//! lio_listio — 批量提交异步 I/O 请求列表
//!
//! 对应 C 源文件: `musl-1.2.6/src/aio/lio_listio.c`
//!
//! 支持同步阻塞等待 (LIO_WAIT) 和异步通知 (LIO_NOWAIT) 两种模式。

#![allow(dead_code, unused_imports, unused_variables)]

use core::ffi::c_int;
use core::ptr;
use core::mem;
use core::slice;
use core::sync::atomic::Ordering;
use core::alloc::Layout;

use crate::import::{self, sigevent, timespec};
use crate::aio::{
    self, aiocb, aio_read, aio_write, aio_error,
    LIO_READ, LIO_WRITE, LIO_NOP, LIO_WAIT, LIO_NOWAIT,
};
use crate::aio_suspend::aio_suspend;

// ============================================================================
// 内部数据结构: LioState
// ============================================================================

/// 封装异步 lio_listio 操作的状态信息，在线程间传递。
///
/// 使用单一堆分配存储结构体及其 cbs 柔性数组，
/// 通过 RAII 模式确保内存安全释放。
struct LioState {
    sev: Option<*mut sigevent>,
    cnt: usize,
    // cbs 柔性数组紧随结构体分配，通过 cbs_ptr_offset() 访问
}

impl LioState {
    /// 分配并初始化 LioState 及其 cbs 柔性数组。
    fn new(cbs_src: *const *mut aiocb, cnt: usize, sev: *mut sigevent) -> Option<Self> {
        // 计算布局: LioState + cnt * sizeof(*const aiocb)
        let header_size = mem::size_of::<LioState>();
        // 对齐到指针大小
        let cbs_offset = header_size;
        let cbs_size = cnt * mem::size_of::<*mut aiocb>();
        let total_size = cbs_offset + cbs_size;

        let ptr = unsafe { import::malloc(total_size) };
        if ptr.is_null() { return None; }

        let sev_ptr = if sev.is_null() { None } else { Some(sev) };

        unsafe {
            // 初始化 LioState 头部
            ptr::write(ptr as *mut LioState, LioState {
                sev: sev_ptr,
                cnt,
            });

            // 拷贝 cbs 数组
            let cbs_dst = (ptr as *mut u8).add(cbs_offset) as *mut *mut aiocb;
            for i in 0..cnt {
                *cbs_dst.add(i) = *cbs_src.add(i);
            }
        }

        Some(unsafe { ptr::read(ptr as *mut LioState) })
    }

    /// 获取 cbs 数组的指针。
    fn cbs_ptr(&self) -> *const *mut aiocb {
        let self_ptr = self as *const Self;
        unsafe { (self_ptr as *const u8).add(mem::size_of::<LioState>()) as *const *mut aiocb }
    }

    /// 获取 cbs 数组的可变指针。
    fn cbs_mut_ptr(&mut self) -> *mut *mut aiocb {
        let self_ptr = self as *mut Self;
        unsafe { (self_ptr as *mut u8).add(mem::size_of::<LioState>()) as *mut *mut aiocb }
    }

    /// 返回 LioState 分配的总大小。
    fn alloc_size(&self) -> usize {
        mem::size_of::<LioState>() + self.cnt * mem::size_of::<*mut aiocb>()
    }
}

impl Drop for LioState {
    fn drop(&mut self) {
        let size = self.alloc_size();
        unsafe { import::free(self as *mut Self as *mut core::ffi::c_void); }
    }
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 判断是否需要为异步操作分配 LioState。
#[inline]
fn needs_async_state(mode: c_int, sev: *mut sigevent) -> bool {
    mode == LIO_WAIT
        || (!sev.is_null() && unsafe { (*sev).sigev_notify != import::SIGEV_NONE })
}

/// 阻塞等待 LioState 中所有异步 I/O 操作完成。
///
/// # Safety
/// 调用方须确保 `st.cbs` 中的非 NULL 条目指向已提交的 aiocb。
unsafe fn lio_wait(st: &mut LioState) -> c_int {
    let cnt = st.cnt;
    let cbs = st.cbs_mut_ptr();
    let mut got_err = false;

    loop {
        let mut i = 0;
        while i < cnt {
            let cb = *cbs.add(i);
            if cb.is_null() {
                i += 1;
                continue;
            }
            let err = aio_error(cb);
            if err == import::EINPROGRESS {
                break; // 还有操作在进行中, 需要继续等待
            }
            if err != 0 {
                got_err = true;
            }
            *cbs.add(i) = ptr::null_mut(); // 标记已处理
            i += 1;
        }
        if i == cnt {
            // 所有操作已完成
            if got_err {
                import::set_errno(import::EIO);
                return -1;
            }
            return 0;
        }
        // 还有操作在进行中 — 挂起等待
        if aio_suspend(cbs as *const *const aiocb, cnt as c_int, ptr::null()) != 0 {
            return -1;
        }
    }
}

/// 向当前进程投递一个异步 I/O 完成信号。
unsafe fn notify_signal(sev: &sigevent) {
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

/// 异步 lio_listio 的工作线程入口。
///
/// 接收 LioState 的所有权，阻塞等待所有 I/O 完成后，
/// 根据 sigev_notify 配置执行信号通知或回调调用。
extern "C" fn wait_thread(p: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    let st = unsafe { &mut *(p as *mut LioState) };

    // 在 lio_wait 前提取 sev 副本 (避免 borrow-after-free)
    let sev_copy = st.sev;

    // 等待所有 I/O 完成
    unsafe { lio_wait(st); }

    // 通知
    if let Some(sev_ptr) = sev_copy {
        let sev = unsafe { &*sev_ptr };
        match sev.sigev_notify {
            import::SIGEV_SIGNAL => unsafe { notify_signal(sev); },
            import::SIGEV_THREAD => {
                if let Some(func) = sev.sigev_notify_function {
                    unsafe { func(core::ptr::read(&sev.sigev_value)); }
                }
            }
            _ => {}
        }
    }

    // LioState 在作用域结束时自动 drop (释放内存)
    // 手动 drop
    unsafe { ptr::drop_in_place(st as *mut LioState); }
    unsafe { import::free(st as *mut LioState as *mut core::ffi::c_void); }

    core::ptr::null_mut()
}

/// 为异步通知路径创建分离工作线程。
///
/// 成功时 LioState 的所有权转移给新线程；失败时 LioState 被 drop。
unsafe fn spawn_wait_thread(mut st: LioState) -> c_int {
    // 提取 sev 中需要的属性信息
    let sev_notify = if let Some(sev_ptr) = st.sev {
        unsafe { (*sev_ptr).sigev_notify }
    } else {
        import::SIGEV_NONE
    };

    // 配置线程属性
    let mut a = import::pthread::pthread_attr_t { _opaque: [0u8; 64] };
    if sev_notify == import::SIGEV_THREAD {
        if let Some(sev_ptr) = st.sev {
            unsafe {
                let attrs = (*sev_ptr).sigev_notify_attributes;
                if !attrs.is_null() {
                    ptr::copy_nonoverlapping(attrs, &raw mut a, 1);
                } else {
                    import::pthread::pthread_attr_init(&raw mut a);
                }
            }
        } else {
            import::pthread::pthread_attr_init(&raw mut a);
        }
    } else {
        import::pthread::pthread_attr_init(&raw mut a);
        // lio_listio 工作线程使用 PAGE_SIZE 栈大小 (不同于 AIO 工作线程)
        import::pthread::pthread_attr_setstacksize(&raw mut a, import::PAGE_SIZE);
        import::pthread::pthread_attr_setguardsize(&raw mut a, 0);
    }
    import::pthread::pthread_attr_setdetachstate(&raw mut a, import::pthread::PTHREAD_CREATE_DETACHED);

    // 阻塞所有信号
    let mut set: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
    let mut set_old: import::sigset_t = import::sigset_t { _opaque: [0u8; 128] };
    import::sigfillset(&raw mut set);
    import::pthread::pthread_sigmask(import::SIG_BLOCK, &raw const set, &raw mut set_old);

    // 将 LioState 转为裸指针 (转移所有权给线程)
    let st_ptr = &raw mut st as *mut LioState;
    // 防止 Drop (所有权已转移给线程)
    mem::forget(st);

    let mut td: import::pthread::pthread_t = import::pthread::pthread_t { _opaque: [0u8; 8] };
    let create_ret = import::pthread::pthread_create(
        &raw mut td,
        &raw const a,
        wait_thread,
        st_ptr as *mut core::ffi::c_void,
    );

    if create_ret != 0 {
        // 线程创建失败 — 回收 LioState 所有权 (手动 drop 并释放)
        ptr::drop_in_place(st_ptr);
        import::free(st_ptr as *mut core::ffi::c_void);
        import::pthread::pthread_sigmask(import::SIG_SETMASK, &raw const set_old, core::ptr::null_mut());
        import::set_errno(import::EAGAIN);
        return -1;
    }

    import::pthread::pthread_sigmask(import::SIG_SETMASK, &raw const set_old, core::ptr::null_mut());
    0
}

// ============================================================================
// 对外导出 API: lio_listio
// ============================================================================

/// 批量发起 I/O 操作列表
///
/// `[Visibility]: User` — 声明于 `<aio.h>`, 符合 POSIX.1-2001/2008 标准。
#[no_mangle]
pub extern "C" fn lio_listio(
    mode: c_int,
    cbs: *const *mut aiocb,
    cnt: c_int,
    sev: *mut sigevent,
) -> c_int {
    // 1. 参数校验
    if cnt < 0 {
        unsafe { import::set_errno(import::EINVAL); }
        return -1;
    }
    let cnt_u = cnt as usize;

    // 2. 判断是否需要分配 LioState
    let need_alloc = needs_async_state(mode, sev);

    let mut st: Option<LioState> = if need_alloc {
        match LioState::new(cbs, cnt_u, sev) {
            Some(st) => Some(st),
            None => {
                unsafe { import::set_errno(import::EAGAIN); }
                return -1;
            }
        }
    } else {
        None
    };

    // 3. 遍历提交各 I/O 操作
    for i in 0..cnt_u {
        let cb = unsafe { *cbs.add(i) };
        if cb.is_null() { continue; }
        let opcode = unsafe { (*cb).aio_lio_opcode };
        let ret = match opcode {
            LIO_READ => aio_read(cb as *mut aiocb),
            LIO_WRITE => aio_write(cb as *mut aiocb),
            _ => 0, // LIO_NOP 或其他 — 跳过
        };
        if ret != 0 {
            // st 在作用域结束时自动 drop
            unsafe { import::set_errno(import::EAGAIN); }
            return -1;
        }
    }

    // 4. 同步等待路径
    if mode == LIO_WAIT {
        let ret = unsafe { lio_wait(st.as_mut().unwrap()) };
        return ret;
    }

    // 5. 异步通知路径
    if let Some(st_val) = st {
        return unsafe { spawn_wait_thread(st_val) };
    }

    // 6. 异步无通知路径 (sev == NULL 或 SIGEV_NONE)
    0
}
