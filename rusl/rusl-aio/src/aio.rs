//! AIO 核心模块 — 异步 I/O 操作控制
//!
//! 本模块实现基于线程的 POSIX 异步 I/O (AIO) 机制。
//! 包含:
//! - 公共类型 `aiocb` 和公共常量
//! - 内部类型 `AioThread`, `AioQueue`, `AioArgs`
//! - 内部函数 `get_queue`, `unref_queue`, `cleanup`, `io_thread_func`, `submit`
//! - 公共 API: `aio_read`, `aio_write`, `aio_fsync`, `aio_return`, `aio_error`, `aio_cancel`
//! - 内部导出符号: `__aio_close`, `__aio_atfork`, `__aio_fut`

#![allow(dead_code, unused_imports, unused_variables)]

use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicI32, AtomicBool, AtomicUsize, Ordering};
use core::ptr;
use core::mem;
use core::cmp;

use crate::import;
use crate::import::pthread as pt;

// ============================================================================
// 内部常量
// ============================================================================

/// MAP 第一级索引大小: (-1u32/2+1)>>24 == 128
const MAP_SIZE_A: usize = 128;
/// MAP 第二至四级各自包含 256 个条目
const LEVEL_SIZE: usize = 256;

// ============================================================================
// 公共数据类型
// ============================================================================

/// POSIX 异步 I/O 控制块
///
/// `[Visibility]: User` — 与 `<aio.h>` 声明的 `struct aiocb` 内存布局严格一致
/// (musl 1.2.6, x86_64: 168 字节).
#[repr(C)]
pub struct aiocb {
    pub aio_fildes: c_int,
    pub aio_lio_opcode: c_int,
    pub aio_reqprio: c_int,
    pub aio_buf: *mut c_void,
    pub aio_nbytes: usize,
    pub aio_sigevent: import::sigevent,
    /// 内部: 所属线程指针
    pub __td: *mut c_void,
    /// 内部: 锁
    pub __lock: [c_int; 2],
    /// 完成状态码 (低 31 位为错误码, bit 31 为"有等待者"标志)
    pub __err: c_int,
    /// 实际 I/O 字节数 (ssize_t)
    pub __ret: isize,
    /// 文件偏移 (off_t)
    pub aio_offset: i64,
    /// 内部链表 next
    pub __next: *mut c_void,
    /// 内部链表 prev
    pub __prev: *mut c_void,
    /// 填充: 32-2*sizeof(void*) = 16 on x86_64
    pub __dummy4: [u8; 16],
}

// ============================================================================
// 公共常量
// ============================================================================

pub const AIO_CANCELED: c_int = 0;
pub const AIO_NOTCANCELED: c_int = 1;
pub const AIO_ALLDONE: c_int = 2;
pub const LIO_READ: c_int = 0;
pub const LIO_WRITE: c_int = 1;
pub const LIO_NOP: c_int = 2;
pub const LIO_WAIT: c_int = 0;
pub const LIO_NOWAIT: c_int = 1;

// ============================================================================
// 内部类型定义
// ============================================================================

pub(crate) struct AioThread {
    pub(crate) td: pt::pthread_t,
    pub(crate) cb: *mut aiocb,
    pub(crate) next: *mut AioThread,
    pub(crate) prev: *mut AioThread,
    pub(crate) q: *mut AioQueue,
    /// 运行状态: 1=运行中, 0=已结束, -1=运行中且有待唤醒者
    pub(crate) running: AtomicI32,
    /// 操作完成后的错误码 (通过 running 同步)
    pub(crate) err: c_int,
    /// 操作类型 (LIO_READ/LIO_WRITE/O_SYNC/O_DSYNC, 提交后不可变)
    pub(crate) op: c_int,
    /// 操作返回值 (读写字节数或 -1)
    pub(crate) ret: isize,
}

pub(crate) struct AioQueue {
    /// 文件描述符 (创建后不可变)
    pub(crate) fd: c_int,
    /// fd 是否可定位
    pub(crate) seekable: bool,
    /// 写操作是否使用追加模式
    pub(crate) append: bool,
    /// 引用计数, >0
    pub(crate) ref_count: AtomicI32,
    /// 是否已初始化 seekable/append
    pub(crate) init: AtomicBool,
    /// 保护队列的互斥锁
    pub(crate) lock: pt::pthread_mutex_t,
    /// 条件变量, 用于有序操作同步
    pub(crate) cond: pt::pthread_cond_t,
    /// 队列链表头指针
    pub(crate) head: *mut AioThread,
}

pub(crate) struct AioArgs {
    pub(crate) cb: *mut aiocb,
    pub(crate) q: *mut AioQueue,
    pub(crate) op: c_int,
    pub(crate) sem: pt::sem_t,
}

// ============================================================================
// 内部全局状态
// ============================================================================

/// 多 aiocb 挂起时的全局 futex 等待字。
/// `[Visibility]: Internal` — 以 C ABI 兼容符号导出.
#[no_mangle]
pub static __aio_fut: AtomicI32 = AtomicI32::new(0);

/// 有未完成 AIO 操作的 fd 数量
pub(crate) static AIO_FD_CNT: AtomicI32 = AtomicI32::new(0);

/// AIO 工作线程栈大小 (首次创建队列时通过 compare_exchange 初始化)
pub(crate) static IO_THREAD_STACK_SIZE: AtomicUsize = AtomicUsize::new(0);

/// 全局读写锁, 保护 MAP 和 AIO_FD_CNT.
/// 全零初始化 == musl PTHREAD_RWLOCK_INITIALIZER.
static mut MAPLOCK: pt::pthread_rwlock_t = pt::pthread_rwlock_t { _opaque: [0u8; 56] };

/// 四级索引表: MAP[a][b][c][d] → *mut AioQueue.
/// 各级均为 calloc 分配的指针数组.
static mut MAP: *mut *mut c_void = core::ptr::null_mut();

// ============================================================================
// 类型转换工具
// ============================================================================

#[inline]
fn ptr_to_pthread_t(p: *mut c_void) -> pt::pthread_t {
    unsafe { mem::transmute::<*mut c_void, pt::pthread_t>(p) }
}

// ============================================================================
// MAP 操作工具函数
// ============================================================================

/// 在 MAP 中查找叶子节点.
/// 调用者必须持有 MAPLOCK (读锁或写锁).
unsafe fn lookup_map(a: usize, b: usize, c: usize, d: usize) -> Option<*mut AioQueue> {
    if MAP.is_null() { return None; }
    let l1 = *MAP.add(a);
    if l1.is_null() { return None; }
    let l2 = *(l1 as *mut *mut c_void).add(b);
    if l2.is_null() { return None; }
    let l3 = *(l2 as *mut *mut c_void).add(c);
    if l3.is_null() { return None; }
    let q = *(l3 as *mut *mut AioQueue).add(d);
    if q.is_null() { return None; }
    Some(q)
}

/// 获取或创建指定 fd 的 AIO 队列.
///
/// - `need = true`: 若队列不存在则创建
/// - `need = false`: 仅查询, 不创建
///
/// 返回时队列的 `lock` 已被调用者持有.
pub(crate) fn get_queue(fd: c_int, need: bool) -> Option<*mut AioQueue> {
    if fd < 0 {
        unsafe { import::set_errno(import::EBADF); }
        return None;
    }

    let a = (fd >> 24) as usize;
    let b = ((fd >> 16) & 0xff) as usize;
    let c = ((fd >> 8) & 0xff) as usize;
    let d = (fd & 0xff) as usize;

    // ---------- 读锁阶段 ----------
    unsafe { pt::pthread_rwlock_rdlock(&raw mut MAPLOCK); }
    let existing = unsafe { lookup_map(a, b, c, d) };
    if existing.is_some() {
        let q = existing.unwrap();
        unsafe { pt::pthread_mutex_lock(&raw mut (*q).lock); }
        unsafe { pt::pthread_rwlock_unlock(&raw mut MAPLOCK); }
        return Some(q);
    }
    if !need {
        unsafe { pt::pthread_rwlock_unlock(&raw mut MAPLOCK); }
        return None;
    }
    unsafe { pt::pthread_rwlock_unlock(&raw mut MAPLOCK); }

    // ---------- 校验 fd ----------
    let getfd = unsafe { import::fcntl(fd, import::F_GETFD) };
    if getfd < 0 {
        return None;
    }

    // ---------- 阻塞信号 + 写锁 ----------
    let mut allmask: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
    let mut origmask: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
    unsafe { import::sigfillset(&raw mut allmask); }
    unsafe { pt::pthread_sigmask(import::SIG_BLOCK, &raw const allmask, &raw mut origmask); }

    unsafe { pt::pthread_rwlock_wrlock(&raw mut MAPLOCK); }

    // ---------- 一次性初始化栈大小 ----------
    if IO_THREAD_STACK_SIZE.load(Ordering::Relaxed) == 0 {
        let val = unsafe { import::__getauxval(import::AT_MINSIGSTKSZ) } as usize;
        let sz = cmp::max(import::MINSIGSTKSZ + 2048, val + 512);
        let _ = IO_THREAD_STACK_SIZE.compare_exchange(0, sz, Ordering::Relaxed, Ordering::Relaxed);
    }

    // ---------- 创建各级索引 ----------
    let result = unsafe {
        // Level 0
        if MAP.is_null() {
            MAP = import::calloc(MAP_SIZE_A, mem::size_of::<*mut c_void>()) as *mut *mut c_void;
            if MAP.is_null() { return unlock_and_signal(&origmask, None); }
        }
        // Level 1
        let l1_ptr = MAP.add(a);
        if (*l1_ptr).is_null() {
            *l1_ptr = import::calloc(LEVEL_SIZE, mem::size_of::<*mut c_void>()) as *mut c_void;
            if (*l1_ptr).is_null() { return unlock_and_signal(&origmask, None); }
        }
        // Level 2
        let l2_base = *l1_ptr as *mut *mut c_void;
        let l2_ptr = l2_base.add(b);
        if (*l2_ptr).is_null() {
            *l2_ptr = import::calloc(LEVEL_SIZE, mem::size_of::<*mut c_void>()) as *mut c_void;
            if (*l2_ptr).is_null() { return unlock_and_signal(&origmask, None); }
        }
        // Level 3
        let l3_base = *l2_ptr as *mut *mut c_void;
        let l3_ptr = l3_base.add(c);
        if (*l3_ptr).is_null() {
            *l3_ptr = import::calloc(LEVEL_SIZE, mem::size_of::<*mut c_void>()) as *mut c_void;
            if (*l3_ptr).is_null() { return unlock_and_signal(&origmask, None); }
        }
        // Level 4 — 叶子节点
        let l4_base = *l3_ptr as *mut *mut AioQueue;
        let l4_ptr = l4_base.add(d);
        if (*l4_ptr).is_null() {
            let q = import::calloc(1, mem::size_of::<AioQueue>()) as *mut AioQueue;
            if q.is_null() { return unlock_and_signal(&origmask, None); }
            (*q).fd = fd;
            (*q).seekable = false;
            (*q).append = false;
            (*q).ref_count = AtomicI32::new(0);
            (*q).init = AtomicBool::new(false);
            (*q).lock = pt::pthread_mutex_t { _opaque: [0u8; 40] };
            (*q).cond = pt::pthread_cond_t { _opaque: [0u8; 48] };
            (*q).head = core::ptr::null_mut();
            pt::pthread_mutex_init(&raw mut (*q).lock, core::ptr::null());
            pt::pthread_cond_init(&raw mut (*q).cond, core::ptr::null());
            *l4_ptr = q;
            AIO_FD_CNT.fetch_add(1, Ordering::Release);
        }
        let q = *l4_ptr;

        // 锁住队列
        pt::pthread_mutex_lock(&raw mut (*q).lock);
        pt::pthread_rwlock_unlock(&raw mut MAPLOCK);
        pt::pthread_sigmask(import::SIG_SETMASK, &raw const origmask, core::ptr::null_mut());
        Some(q)
    };
    result
}

/// 清理: 释放 MAPLOCK 写锁 + 恢复信号掩码, 返回 None.
unsafe fn unlock_and_signal(origmask: &import::sigset_t, _q: Option<*mut AioQueue>) -> Option<*mut AioQueue> {
    pt::pthread_rwlock_unlock(&raw mut MAPLOCK);
    pt::pthread_sigmask(import::SIG_SETMASK, origmask, core::ptr::null_mut());
    None
}

// ============================================================================
// unref_queue
// ============================================================================

/// 安全地释放对队列的引用.
/// 调用者必须持有 `q.lock`.
pub(crate) fn unref_queue(q: *mut AioQueue) {
    unsafe {
        let qr = &mut *q;
        if qr.ref_count.load(Ordering::Relaxed) > 1 {
            qr.ref_count.fetch_sub(1, Ordering::Release);
            pt::pthread_mutex_unlock(&raw mut qr.lock);
            return;
        }
        // 可能是最后引用 — 乐观锁策略
        pt::pthread_mutex_unlock(&raw mut qr.lock);
        pt::pthread_rwlock_wrlock(&raw mut MAPLOCK);
        pt::pthread_mutex_lock(&raw mut qr.lock);

        if qr.ref_count.load(Ordering::Relaxed) == 1 {
            // 确认无新引用 — 从 MAP 移除
            let fd = qr.fd;
            let a = (fd >> 24) as usize;
            let b = ((fd >> 16) & 0xff) as usize;
            let c = ((fd >> 8) & 0xff) as usize;
            let d = (fd & 0xff) as usize;

            if !MAP.is_null() {
                let l1 = *MAP.add(a);
                if !l1.is_null() {
                    let l2 = *(l1 as *mut *mut c_void).add(b);
                    if !l2.is_null() {
                        let l3 = *(l2 as *mut *mut c_void).add(c);
                        if !l3.is_null() {
                            *(l3 as *mut *mut AioQueue).add(d) = core::ptr::null_mut();
                        }
                    }
                }
            }
            AIO_FD_CNT.fetch_sub(1, Ordering::Release);
            pt::pthread_rwlock_unlock(&raw mut MAPLOCK);
            pt::pthread_mutex_unlock(&raw mut qr.lock);
            import::free(q as *mut c_void);
        } else {
            qr.ref_count.fetch_sub(1, Ordering::Release);
            pt::pthread_rwlock_unlock(&raw mut MAPLOCK);
            pt::pthread_mutex_unlock(&raw mut qr.lock);
        }
    }
}

// ============================================================================
// cleanup — AIO 操作完成的统一清理路径
// ============================================================================

extern "C" fn cleanup(ctx: *mut c_void) {
    unsafe {
    let at = &mut *(ctx as *mut AioThread);
    let q = at.q;
    let cb = at.cb;

    // 复制 sigevent (在获取队列锁之前)
    let sev = ptr::read(&(*cb).aio_sigevent);
    let sev_notify = sev.sigev_notify;

    // 1. 设置操作返回值
    (*cb).__ret = at.ret;

    // 2. 通知 aio_cancel/close 等待者
    let old_running = at.running.swap(0, Ordering::AcqRel);
    if old_running == -1 {
        import::__wake(&raw const at.running as *const c_int, -1);
    }

    // 3. 通知单 aiocb 的 aio_suspend 等待者
    //    __err 为 c_int, 通过裸指针转为 AtomicI32 执行原子 swap
    let err_atomic = &raw mut (*cb).__err as *mut AtomicI32;
    let old_err = (*err_atomic).swap(at.err, Ordering::AcqRel);
    if old_err != import::EINPROGRESS {
        import::__wake(&raw const (*cb).__err, -1);
    }

    // 4. 通知多 aiocb 的 aio_suspend 列表等待者
    let old_fut = __aio_fut.swap(0, Ordering::AcqRel);
    if old_fut != 0 {
        import::__wake(&raw const __aio_fut as *const c_int, -1);
    }

    // 5. 链表移除与条件变量广播
    pt::pthread_mutex_lock(&raw mut (*q).lock);

    if !at.next.is_null() { (*at.next).prev = at.prev; }
    if !at.prev.is_null() { (*at.prev).next = at.next; }
    else { (*q).head = at.next; }

    pt::pthread_cond_broadcast(&raw mut (*q).cond);

    unref_queue(q);

    // 6. SIGEV 通知
    if sev_notify == import::SIGEV_SIGNAL {
        let si = import::siginfo_t {
            si_signo: sev.sigev_signo,
            si_errno: 0,
            si_code: import::SI_ASYNCIO,
            si_pid: import::getpid(),
            si_uid: import::getuid(),
            si_value: ptr::read(&sev.sigev_value),
            __pad: [0i8; 88],
        };
        let _ = rusl_syscall::do_syscall!(
            import::SYS_rt_sigqueueinfo,
            si.si_pid as i64,
            si.si_signo as i64,
            &raw const si as i64
        );
    }

    if sev_notify == import::SIGEV_THREAD {
        // 重置取消状态, 防止通知回调被取消
        let mut oldstate: c_int = 0;
        pt::pthread_setcancelstate(pt::PTHREAD_CANCEL_DISABLE, &raw mut oldstate);
        if let Some(func) = sev.sigev_notify_function {
            func(ptr::read(&sev.sigev_value));
        }
    }
    } // end unsafe block
}

// ============================================================================
// io_thread_func — AIO 工作线程主函数
// ============================================================================

extern "C" fn io_thread_func(ctx: *mut c_void) -> *mut c_void {
    unsafe {
    // 从原始指针读取参数 (不通过 Box, 见 submit 注释)
    let args = ctx as *mut AioArgs;
    let cb = (*args).cb;
    let fd = (*cb).aio_fildes;
    let op = (*args).op;
    let buf = (*cb).aio_buf;
    let len = (*cb).aio_nbytes;
    let off = (*cb).aio_offset;
    let q = (*args).q;

    // 获取队列锁
    pt::pthread_mutex_lock(&raw mut (*q).lock);

    // 通知 submit() 线程可以安全返回
    pt::sem_post(&raw mut (*args).sem);

    // 构建栈上 AioThread
    let mut at = AioThread {
        td: pt::pthread_t { _opaque: [0u8; 8] },
        cb,
        next: core::ptr::null_mut(),
        prev: core::ptr::null_mut(),
        q,
        running: AtomicI32::new(1),
        err: import::ECANCELED,
        op,
        ret: -1,
    };
    at.td = ptr_to_pthread_t(pt::pthread_self());

    // 链入队列头部
    at.prev = core::ptr::null_mut();
    at.next = (*q).head;
    if !at.next.is_null() {
        (*at.next).prev = &raw mut at;
    }
    (*q).head = &raw mut at;

    // 首次访问 — 初始化 fd 属性
    if !(*q).init.load(Ordering::Relaxed) {
        let seekable = import::lseek(fd, 0, import::SEEK_CUR) >= 0;
        (*q).seekable = seekable;
        let flags = import::fcntl(fd, import::F_GETFL);
        (*q).append = !seekable || (flags & import::O_APPEND) != 0;
        (*q).init.store(true, Ordering::Release);
    }

    // 注册清理回调
    let mut ptcb = pt::__ptcb {
        __next: core::ptr::null_mut(),
        __f: Some(cleanup),
        __x: &raw mut at as *mut c_void,
    };
    pt::_pthread_cleanup_push(&raw mut ptcb, cleanup, &raw mut at as *mut c_void);

    // 有序操作同步
    if op != LIO_READ && (op != LIO_WRITE || (*q).append) {
        loop {
            let mut p = at.next;
            while !p.is_null() && (*p).op != LIO_WRITE {
                p = (*p).next;
            }
            if p.is_null() { break; }
            pt::pthread_cond_wait(&raw mut (*q).cond, &raw mut (*q).lock);
        }
    }

    // 释放队列锁, 执行 I/O
    pt::pthread_mutex_unlock(&raw mut (*q).lock);

    let ret: isize = match op {
        LIO_WRITE => {
            if (*q).append {
                import::write(fd, buf as *const c_void, len)
            } else {
                import::pwrite(fd, buf as *const c_void, len, off)
            }
        }
        LIO_READ => {
            if !(*q).seekable {
                import::read(fd, buf, len)
            } else {
                import::pread(fd, buf, len, off)
            }
        }
        import::O_SYNC => import::fsync(fd) as isize,
        import::O_DSYNC => import::fdatasync(fd) as isize,
        _ => -1,
    };

    at.ret = ret;
    at.err = if ret < 0 {
        unsafe { *import::__errno_location() }
    } else {
        0
    };

    // 执行清理 (execute = 1 → 调用 cleanup)
    pt::_pthread_cleanup_pop(&raw mut ptcb, 1);

    // 释放 AioArgs 内存 (由 submit 分配)
    import::free(ctx);
    } // end unsafe block

    core::ptr::null_mut()
}

// ============================================================================
// submit — 统一的 AIO 提交入口
// ============================================================================

fn submit(cb: *mut aiocb, op: c_int) -> c_int {
    unsafe {
        let fd = (*cb).aio_fildes;

        // 1. 获取/创建队列
        let q_opt = get_queue(fd, true);
        if q_opt.is_none() {
            let err = *import::__errno_location();
            if err != import::EBADF {
                import::set_errno(import::EAGAIN);
            }
            (*cb).__ret = -1;
            (*cb).__err = *import::__errno_location();
            return -1;
        }
        let q = q_opt.unwrap();

        (*q).ref_count.fetch_add(1, Ordering::Release);
        pt::pthread_mutex_unlock(&raw mut (*q).lock);

        // 2. 分配 AioArgs (使用 malloc 而非 Box, 以便 sem_wait/sem_post 安全共享)
        let args_ptr = import::malloc(mem::size_of::<AioArgs>()) as *mut AioArgs;
        if args_ptr.is_null() {
            pt::pthread_mutex_lock(&raw mut (*q).lock);
            unref_queue(q);
            (*cb).__err = import::EAGAIN;
            import::set_errno(import::EAGAIN);
            (*cb).__ret = -1;
            return -1;
        }
        ptr::write(&raw mut (*args_ptr).cb, cb);
        ptr::write(&raw mut (*args_ptr).q, q);
        ptr::write(&raw mut (*args_ptr).op, op);
        ptr::write(&raw mut (*args_ptr).sem, pt::sem_t { _opaque: [0u8; 32] });
        pt::sem_init(&raw mut (*args_ptr).sem, 0, 0);

        // 3. 配置线程属性
        let mut a = pt::pthread_attr_t { _opaque: [0u8; 64] };
        let sev_notify = (*cb).aio_sigevent.sigev_notify;
        if sev_notify == import::SIGEV_THREAD {
            let attrs = (*cb).aio_sigevent.sigev_notify_attributes;
            if !attrs.is_null() {
                ptr::copy_nonoverlapping(attrs, &raw mut a, 1);
            } else {
                pt::pthread_attr_init(&raw mut a);
            }
        } else {
            pt::pthread_attr_init(&raw mut a);
            let stack = IO_THREAD_STACK_SIZE.load(Ordering::Relaxed);
            if stack > 0 { pt::pthread_attr_setstacksize(&raw mut a, stack); }
            pt::pthread_attr_setguardsize(&raw mut a, 0);
        }
        pt::pthread_attr_setdetachstate(&raw mut a, pt::PTHREAD_CREATE_DETACHED);

        // 4. 阻塞所有信号
        let mut allmask: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
        let mut origmask: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
        import::sigfillset(&raw mut allmask);
        pt::pthread_sigmask(import::SIG_BLOCK, &raw const allmask, &raw mut origmask);

        (*cb).__err = import::EINPROGRESS;

        // 5. 创建线程
        let mut td: pt::pthread_t = pt::pthread_t { _opaque: [0u8; 8] };
        let create_ret = pt::pthread_create(
            &raw mut td,
            &raw const a,
            io_thread_func,
            args_ptr as *mut c_void,
        );

        let mut ret: c_int = 0;
        if create_ret != 0 {
            // 线程创建失败 — 释放资源
            import::free(args_ptr as *mut c_void);
            pt::pthread_mutex_lock(&raw mut (*q).lock);
            unref_queue(q);
            (*cb).__err = import::EAGAIN;
            import::set_errno(import::EAGAIN);
            (*cb).__ret = -1;
            ret = -1;
        }

        // 6. 恢复信号掩码
        pt::pthread_sigmask(import::SIG_SETMASK, &raw const origmask, core::ptr::null_mut());

        // 7. 等待工作线程获取队列锁 (通过信号量同步)
        if ret == 0 {
            // 工作线程在获取 q.lock 后立即 sem_post
            // 此后工作线程不再访问 AioArgs (除 sem 相关), submit 继续
            while pt::sem_wait(&raw mut (*args_ptr).sem) != 0 {
                // 可能被信号中断, 重试
            }
            // 此时工作线程已获取 q.lock, AioArgs 所有权已移交
            // 工作线程稍后通过 import::free(args_ptr) 释放内存
        }

        ret
    }
}

// ============================================================================
// 对外导出 API
// ============================================================================

#[no_mangle]
pub extern "C" fn aio_read(cb: *mut aiocb) -> c_int { submit(cb, LIO_READ) }

#[no_mangle]
pub extern "C" fn aio_write(cb: *mut aiocb) -> c_int { submit(cb, LIO_WRITE) }

#[no_mangle]
pub extern "C" fn aio_fsync(op: c_int, cb: *mut aiocb) -> c_int {
    if op != import::O_SYNC && op != import::O_DSYNC {
        unsafe { import::set_errno(import::EINVAL); }
        return -1;
    }
    submit(cb, op)
}

#[no_mangle]
pub extern "C" fn aio_return(cb: *mut aiocb) -> isize {
    unsafe { (*cb).__ret }
}

#[no_mangle]
pub extern "C" fn aio_error(cb: *const aiocb) -> c_int {
    core::sync::atomic::fence(Ordering::Acquire);
    unsafe { (*cb).__err & 0x7fffffff }
}

#[no_mangle]
pub extern "C" fn aio_cancel(fd: c_int, cb: *mut aiocb) -> c_int {
    unsafe {
        if !cb.is_null() && (*cb).aio_fildes != fd {
            import::set_errno(import::EINVAL);
            return -1;
        }

        let mut ret = AIO_ALLDONE;

        let mut allmask: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
        let mut origmask: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
        import::sigfillset(&raw mut allmask);
        pt::pthread_sigmask(import::SIG_BLOCK, &raw const allmask, &raw mut origmask);

        let q_opt = get_queue(fd, false);
        if q_opt.is_none() {
            if *import::__errno_location() == import::EBADF { ret = -1; }
            pt::pthread_sigmask(import::SIG_SETMASK, &raw const origmask, core::ptr::null_mut());
            return ret;
        }
        let q = q_opt.unwrap();

        let mut p = (*q).head;
        while !p.is_null() {
            if !cb.is_null() && cb != (*p).cb {
                p = (*p).next;
                continue;
            }
            // CAS: running 1 → -1 (标记为"运行中且有待取消者")
            if (*p).running.compare_exchange(1, -1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                pt::pthread_cancel(ptr::read(&(*p).td));
                import::__wait(
                    &raw const (*p).running as *const c_int,
                    core::ptr::null(),
                    -1, 1,
                );
                if (*p).err == import::ECANCELED { ret = AIO_CANCELED; }
            }
            p = (*p).next;
        }

        pt::pthread_mutex_unlock(&raw mut (*q).lock);
        pt::pthread_sigmask(import::SIG_SETMASK, &raw const origmask, core::ptr::null_mut());
        ret
    }
}

// ============================================================================
// 内部导出符号 (跨模块可见, 需 ABI 兼容)
// ============================================================================

#[no_mangle]
pub extern "C" fn __aio_close(fd: c_int) -> c_int {
    core::sync::atomic::fence(Ordering::Acquire);
    if AIO_FD_CNT.load(Ordering::Acquire) != 0 {
        aio_cancel(fd, core::ptr::null_mut());
    }
    fd
}

#[no_mangle]
pub extern "C" fn __aio_atfork(who: c_int) {
    unsafe {
        if who < 0 {
            // fork 前: 获取读锁阻塞 MAP 修改
            pt::pthread_rwlock_rdlock(&raw mut MAPLOCK);
        } else if who == 0 {
            // 父进程恢复: 释放读锁
            pt::pthread_rwlock_unlock(&raw mut MAPLOCK);
        } else {
            // 子进程清理
            AIO_FD_CNT.store(0, Ordering::Release);

            // 尝试获取读锁 (tryrdlock):
            // - 成功: 遍历清空 MAP 叶子节点, 重新初始化 rwlock
            // - 失败: 直接清空 MAP 指针 (_Fork 场景)
            if pt::pthread_rwlock_tryrdlock(&raw mut MAPLOCK) == 0 {
                if !MAP.is_null() {
                    for ai in 0..MAP_SIZE_A {
                        let l1 = *MAP.add(ai);
                        if l1.is_null() { continue; }
                        for bi in 0..LEVEL_SIZE {
                            let l2 = *(l1 as *mut *mut c_void).add(bi);
                            if l2.is_null() { continue; }
                            for ci in 0..LEVEL_SIZE {
                                let l3 = *(l2 as *mut *mut c_void).add(ci);
                                if l3.is_null() { continue; }
                                for di in 0..LEVEL_SIZE {
                                    *(l3 as *mut *mut AioQueue).add(di) = core::ptr::null_mut();
                                }
                            }
                        }
                    }
                }
                // 重新初始化 rwlock (当前线程不是锁持有者)
                pt::pthread_rwlock_init(&raw mut MAPLOCK, core::ptr::null());
            } else {
                MAP = core::ptr::null_mut();
            }
        }
    }
}
