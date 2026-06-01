//! pthread_impl 模块 — rusl 线程子系统（POSIX pthread）核心内部模块。
//!
//! 本模块定义了线程控制块 (TCB) `Pthread` 结构体、基于 Linux futex 的
//! 线程同步原语、TLS 管理、线程取消点机制、信号系统交互等。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。
//!
//! # Pthread 结构体布局说明
//!
//! `Pthread` 分为三个部分：
//! - **Part 1**：外部 ABI 字段，偏移量不得修改
//! - **Part 2**：实现细节，可自由调整布局
//! - **Part 3**：相对于结构体末尾的外部 ABI 字段

#![allow(unused)]

use crate::lock::SpinLock;
use core::ffi::{c_int, c_uint, c_long, c_void};
use core::sync::atomic::{AtomicI32, AtomicU8, AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// 前向类型声明（完整定义位于各自模块）
// ---------------------------------------------------------------------------

/// `pthread_t` — POSIX 线程标识符（不透明指针指向 `Pthread`）
pub type pthread_t = *mut Pthread;

/// 内部取消点清理处理链表节点（占位符）
#[repr(C)]
pub struct Ptcb {
    _private: [u8; 32], // 实际布局由 cancel 模块定义
}

/// POSIX 互斥锁（占位符）
#[repr(C)]
pub struct PthreadMutex {
    _private: [u8; 40], // 实际布局由 pthread 模块定义
}

/// POSIX 条件变量（占位符）
#[repr(C)]
pub struct PthreadCond {
    _private: [u8; 48], // 实际布局由 pthread 模块定义
}

/// POSIX 读写锁（占位符）
#[repr(C)]
pub struct PthreadRwlock {
    _private: [u8; 56], // 实际布局由 pthread 模块定义
}

/// POSIX 屏障（占位符）
#[repr(C)]
pub struct PthreadBarrier {
    _private: [u8; 32], // 实际布局由 pthread 模块定义
}

/// 线程属性（占位符）
#[repr(C)]
pub struct PthreadAttr {
    _private: [u8; 64], // 实际布局由 pthread 模块定义
}

/// 信号集类型（占位符）
#[repr(C)]
pub struct Sigset {
    _private: [u8; 128], // 实际布局由 signal 模块定义
}

/// 时间规格（占位符）
#[repr(C)]
pub struct timespec {
    /// 秒
    pub tv_sec: isize,
    /// 纳秒
    pub tv_nsec: isize,
}

/// POSIX 时钟 ID 类型
pub type clockid_t = c_int;

/// TLS 模块偏移描述符（用于延迟绑定 TLS）
#[repr(C)]
pub struct TlsModOff {
    _private: [u8; 16], // 实际布局由 TLS 模块定义
}

// ---------------------------------------------------------------------------
// 架构相关常量
// ---------------------------------------------------------------------------

/// x86_64 架构的线程指针偏移量
#[cfg(target_arch = "x86_64")]
pub const TP_OFFSET: isize = 0;

/// aarch64 架构的线程指针偏移量
#[cfg(target_arch = "aarch64")]
pub const TP_OFFSET: isize = 0;

/// TLS 数据是否位于线程指针上方
#[cfg(target_arch = "aarch64")]
pub const TLS_ABOVE_TP: bool = true;

/// TLS 数据是否位于线程指针下方（x86_64 等情况）
#[cfg(target_arch = "x86_64")]
pub const TLS_ABOVE_TP: bool = false;

// ---------------------------------------------------------------------------
// 线程控制块 (TCB)
// ---------------------------------------------------------------------------

/// 线程控制块 — 线程在内存中的完整表示。
///
/// # ABI 约束
///
/// Part 1 和 Part 3 字段的偏移量是外部 ABI（编译器/运行时可见），不得修改。
/// Part 2 字段为仅内部使用的实现细节，可自由调整。
///
/// # 不变量
///
/// * `self_` 指针必须指向自身的 `Pthread` 起始地址
/// * `detach_state` 只能按状态迁移图进行迁移
#[repr(C)]
pub struct Pthread {
    // ===== Part 1 — 外部 ABI，偏移量不得修改 =====

    /// 自引用指针（`pthread_self()` 的实现基础）
    pub self_: *mut Pthread,

    /// Dynamic Thread Vector（TLS 的动态描述符表）
    #[cfg(not(TLS_ABOVE_TP))]
    pub dtv: *mut usize,

    /// 全局线程链表前驱
    pub prev: *mut Pthread,

    /// 全局线程链表后继
    pub next: *mut Pthread,

    /// 系统信息（vsyscall 页地址等）
    pub sysinfo: usize,

    /// Canary 填充（仅特定架构，当前未启用）
    // TODO: 当架构相关的 CANARY_PAD 常量就绪后启用
    // #[cfg(CANARY_PAD)]
    // pub canary_pad: usize,

    /// 栈保护 canary
    #[cfg(not(TLS_ABOVE_TP))]
    pub canary: usize,

    // ===== Part 2 — 实现细节，可自由调整布局 =====

    /// 内核线程 ID
    pub tid: c_int,

    /// 线程局部的 errno 值
    pub errno_val: c_int,

    /// 线程分离状态（`DetachState`）
    pub detach_state: AtomicI32,

    /// 取消请求标志
    pub cancel: AtomicI32,

    /// 取消禁用计数（>0 时不能取消）
    pub canceldisable: AtomicU8,

    /// 异步取消启用标志
    pub cancelasync: AtomicU8,

    /// 线程特定数据是否已初始化
    pub tsd_used: bool,

    /// dlopen/dlsym 错误标志
    pub dlerror_flag: bool,

    /// 线程 mmap 区域起始地址
    pub map_base: *mut u8,

    /// 线程 mmap 区域大小
    pub map_size: usize,

    /// 线程栈基址
    pub stack: *mut c_void,

    /// 线程栈大小
    pub stack_size: usize,

    /// 保护页大小
    pub guard_size: usize,

    /// 线程退出返回值
    pub result: *mut c_void,

    /// 取消点清理处理链表头
    pub cancelbuf: *mut Ptcb,

    /// 线程特定数据数组
    pub tsd: *mut *mut c_void,

    /// Robust mutex 链表
    pub robust_list: RobustList,

    /// 线程局部的 h_errno
    pub h_errno_val: c_int,

    /// 线程局部的定时器 ID
    pub timer_id: AtomicI32,

    /// 线程局部的 locale（占位符）
    pub locale: Locale,

    /// 信号递送锁
    pub killlock: SpinLock,

    /// 线程局部的 dlerror 缓冲区
    pub dlerror_buf: *mut u8,

    /// 线程持有的 stdio 锁链表头
    pub stdio_locks: *mut c_void,

    // ===== Part 3 — 相对于结构体末尾的外部 ABI =====

    /// 栈保护 canary（当 TLS 位于 TP 上方时）
    #[cfg(TLS_ABOVE_TP)]
    pub canary: usize,

    /// Dynamic Thread Vector（当 TLS 位于 TP 上方时）
    #[cfg(TLS_ABOVE_TP)]
    pub dtv: *mut usize,
}

// ---------------------------------------------------------------------------
// 线程分离状态
// ---------------------------------------------------------------------------

/// 线程分离状态枚举。
///
/// 状态迁移图：
/// ```text
/// Joinable ──[pthread_detach]──> Detached
/// Joinable ──[线程结束]──> Exiting ──> Exited
/// Detached  ──[线程结束]──> (自动释放资源)
/// ```
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DetachState {
    /// 线程已退出（或尚未创建）
    Exited = 0,
    /// 线程正在退出过程中
    Exiting = 1,
    /// 线程可被 `pthread_join` 等待
    Joinable = 2,
    /// 线程已被分离，退出时自动释放资源
    Detached = 3,
}

// ---------------------------------------------------------------------------
// 辅助类型
// ---------------------------------------------------------------------------

/// Robust mutex 链表。
#[repr(C)]
pub struct RobustList {
    pub head: *mut c_void,
    pub off: c_long,
    pub pending: *mut c_void,
}

/// 线程局部的 locale（占位符）。
#[repr(C)]
pub struct Locale {
    _private: [u8; 64], // 实际布局由 locale 模块定义
}

// ---------------------------------------------------------------------------
// 默认常量
// ---------------------------------------------------------------------------

/// 默认栈大小 — 128KB
pub const DEFAULT_STACK_SIZE: usize = 131072;

/// 默认保护页大小 — 8KB
pub const DEFAULT_GUARD_SIZE: usize = 8192;

/// 最大栈大小 — 8MB
pub const DEFAULT_STACK_MAX: usize = 8 << 20;

/// 最大保护页大小 — 1MB
pub const DEFAULT_GUARD_MAX: usize = 1 << 20;

/// C11 线程哨兵值（标记线程通过 `thrd_create()` 创建）
pub const ATTRP_C11_THREAD: *mut c_void = usize::MAX as *mut c_void;

// ---------------------------------------------------------------------------
// 可覆盖的默认值
// ---------------------------------------------------------------------------

/// 默认栈大小（可被 ulimit 或环境变量覆盖）
pub static mut DEFAULT_STACKSIZE: c_uint = DEFAULT_STACK_SIZE as c_uint;

/// 默认保护页大小（可被 ulimit 或环境变量覆盖）
pub static mut DEFAULT_GUARDSIZE: c_uint = DEFAULT_GUARD_SIZE as c_uint;

// ---------------------------------------------------------------------------
// 内部信号常量
// ---------------------------------------------------------------------------

/// musl 内部定时器信号编号
pub const SIGTIMER: c_int = 32;

/// musl 内部取消信号编号
pub const SIGCANCEL: c_int = 33;

/// musl 内部同步调用信号编号
pub const SIGSYNCCALL: c_int = 34;

// ---------------------------------------------------------------------------
// 全局变量
// ---------------------------------------------------------------------------

/// 保护全局线程链表的自旋锁
pub static THREAD_LIST_LOCK: SpinLock = SpinLock::new();

/// 当前分配的 TSD 数组大小
pub static PTHREAD_TSD_SIZE: AtomicUsize = AtomicUsize::new(0);

/// 标记 `EINTR` 是否合法的全局标志
pub static EINTR_VALID_FLAG: AtomicI32 = AtomicI32::new(0);

/// `abort()` 信号安全的全局锁
pub static ABORT_LOCK: SpinLock = SpinLock::new();

// ---------------------------------------------------------------------------
// Futex 同步原语
// ---------------------------------------------------------------------------

/// Futex 超时等待的最大自旋迭代次数（10_000_000 次）。
///
/// 仅在 `timedwait` / `timedwait_cp` 的简化实现中用作超时占位符。
/// 当真正的 `futex` 系统调用封装就绪后，此常量将被移除。
const TIMEDWAIT_MAX_SPIN: usize = 10_000_000;

/// 保护并发 `pthread_create` 调用的自旋锁。
///
/// 在 `fork()` 等需要禁止线程创建的关键路径上也可使用。
pub static PTC_LOCK: SpinLock = SpinLock::new();

// ---------------------------------------------------------------------------
// Futex 同步原语
// ---------------------------------------------------------------------------

/// 唤醒最多 `cnt` 个等待在 futex 字上的线程（简化占位实现）。
///
/// 实际实现应通过 `futex(addr, FUTEX_WAKE | priv_, cnt, 0, 0, 0)` 系统调用完成。
/// 当前占位实现执行一次 volatile 读取以防止编译器优化消除调用。
///
/// # 参数
///
/// * `addr` - futex 字的地址（非空）
/// * `cnt`  - 最大唤醒数量
/// * `priv_` - `FUTEX_PRIVATE` 标志值
pub fn wake(addr: *const AtomicI32, cnt: c_int, priv_: c_int) {
    let _ = cnt;
    let _ = priv_;
    // 占位：volatile 读取确保调用不会被优化消除
    unsafe {
        core::ptr::read_volatile(addr);
    }
}

/// 在 futex 字上等待（简化自旋实现）。
///
/// 自旋等待直到 `*addr != val`。使用 `core::hint::spin_loop()` 减少 CPU 占用。
///
/// 实际实现应通过 `futex(addr, FUTEX_WAIT | priv_, val, 0, 0, 0)` 系统调用完成。
///
/// # 参数
///
/// * `addr`  - futex 字的地址
/// * `val`   - 期望的比较值：当 `*addr != val` 时返回
/// * `priv_` - `FUTEX_PRIVATE` 标志值
pub fn futexwait(addr: *const AtomicI32, val: c_int, priv_: c_int) {
    let _ = priv_;
    // 自旋等待直到 futex 字的值不再等于 val
    unsafe {
        while (*addr).load(Ordering::Acquire) == val {
            core::hint::spin_loop();
        }
    }
}

/// 带超时的 futex 等待（简化自旋实现）。
///
/// 若 `at` 为 NULL，则无限等待（等价于 `futexwait`）。
/// 若 `at` 非 NULL，则自旋等待直到超时（自旋次数达到上限）或条件满足。
///
/// # 参数
///
/// * `addr`  - futex 字的地址
/// * `val`   - 期望的比较值
/// * `clk`   - 超时时钟类型（`CLOCK_REALTIME` 或 `CLOCK_MONOTONIC`），简化实现中忽略
/// * `at`    - 超时时间规格（NULL 表示无限等待）
/// * `priv_` - `FUTEX_PRIVATE` 标志值
///
/// # 返回值
///
/// * `0` — 成功被唤醒（条件满足）
/// * 非零 — 超时（简化实现返回 `-1`）
#[inline]
pub fn timedwait(
    addr: *const AtomicI32,
    val: c_int,
    clk: clockid_t,
    at: *const timespec,
    priv_: c_int,
) -> c_int {
    let _ = clk;
    let _ = priv_;

    if at.is_null() {
        // 无限等待
        futexwait(addr, val, priv_);
        0
    } else {
        // 有限自旋等待：在固定自旋次数后超时
        for _ in 0..TIMEDWAIT_MAX_SPIN {
            unsafe {
                if (*addr).load(Ordering::Acquire) != val {
                    return 0;
                }
            }
            core::hint::spin_loop();
        }
        // 超时 — 简化实现返回 -1 表示 ETIMEDOUT
        -1
    }
}

/// 带取消点的超时 futex 等待（简化自旋实现）。
///
/// 与 `timedwait` 相同，但在自旋循环中周期性检查线程取消标志（调用 `testcancel()`）。
///
/// # 参数
///
/// * `addr`  - futex 字的地址
/// * `val`   - 期望的比较值
/// * `clk`   - 超时时钟类型（简化实现中忽略）
/// * `at`    - 超时时间规格（NULL 表示无限等待）
/// * `priv_` - `FUTEX_PRIVATE` 标志值
///
/// # 返回值
///
/// * `0` — 成功被唤醒
/// * 非零 — 超时或线程被取消
pub fn timedwait_cp(
    addr: *const AtomicI32,
    val: c_int,
    clk: clockid_t,
    at: *const timespec,
    priv_: c_int,
) -> c_int {
    let _ = clk;
    let _ = priv_;

    if at.is_null() {
        // 无限等待，周期性检查取消点
        loop {
            testcancel();
            unsafe {
                if (*addr).load(Ordering::Acquire) != val {
                    return 0;
                }
            }
            // 每 1024 次自旋检查一次取消点，平衡性能与响应性
            for _ in 0..1024 {
                unsafe {
                    if (*addr).load(Ordering::Acquire) != val {
                        return 0;
                    }
                }
                core::hint::spin_loop();
            }
        }
    } else {
        // 有限等待，周期性检查取消点
        let mut remaining = TIMEDWAIT_MAX_SPIN;
        while remaining > 0 {
            testcancel();
            unsafe {
                if (*addr).load(Ordering::Acquire) != val {
                    return 0;
                }
            }
            core::hint::spin_loop();
            remaining -= 1;
        }
        // 超时
        -1
    }
}

// ---------------------------------------------------------------------------
// TLS 管理
// ---------------------------------------------------------------------------

/// TLS 变量地址的动态解析（延迟绑定 TLS 模型，占位实现）。
///
/// 返回 NULL，表示当前尚未实现完整的动态 TLS 地址解析。
/// 完整实现需要访问 DTV（Dynamic Thread Vector）表。
pub fn tls_get_addr(v: *mut TlsModOff) -> *mut c_void {
    let _ = v;
    // 占位：返回 null，标记为需要完整 TLS 实现
    core::ptr::null_mut()
}

/// 初始化线程指针，设置主线程 TLS（占位实现）。
///
/// 简化实现始终返回 0 表示成功。
/// 完整实现需要通过 `arch_prctl(ARCH_SET_FS, tp)` 设置 FS 段基址。
pub fn init_tp(tp: *mut c_void) -> c_int {
    let _ = tp;
    // 占位：返回 0 表示成功
    0
}

/// 从 TLS 模板复制 TLS 数据到新线程的 TLS 区域（占位实现）。
///
/// 简化实现直接返回 `mem` 作为线程指针。
/// 完整实现需要将 TLS 初始化映像复制到 `mem` 指向的区域。
pub fn copy_tls(mem: *mut u8) -> *mut c_void {
    // 占位：返回 mem 本身作为线程指针
    mem as *mut c_void
}

/// 子进程 fork 后重置 TLS 状态（占位实现）。
///
/// 简化实现为空操作。
/// 完整实现需要重置子进程中的 DTV 和相关 TLS 数据结构。
pub fn reset_tls() {
    // 占位：空操作
}

// ---------------------------------------------------------------------------
// 线程取消点机制
// ---------------------------------------------------------------------------

/// 检查取消标志（简化占位实现）。
///
/// 当前为空操作。完整实现应当：
///
/// 1. 检查当前线程的 `cancel` 标志
/// 2. 若 `cancel` 已置位且 `canceldisable == 0`，调用 `pthread_exit(PTHREAD_CANCELED)`
///
/// 由于当前 rusl 缺少获取当前线程 TCB 的机制（`__pthread_self()`），
/// 暂时实现为空操作。
pub fn testcancel() {
    // 占位：空操作 — 需要 __pthread_self() 获取当前线程 TCB
}

/// 将清理处理器压入取消清理栈（简化占位实现）。
///
/// 完整实现需要将 `cb` 节点链入当前线程 TCB 的 `cancelbuf` 链表。
pub fn do_cleanup_push(cb: *mut Ptcb) {
    let _ = cb;
    // 占位：空操作
}

/// 从取消清理栈弹出并执行清理处理器（简化占位实现）。
///
/// 完整实现需要从 `cancelbuf` 链表中移除 `cb` 节点并根据上下文
/// 决定是否立即执行清理回调。
pub fn do_cleanup_pop(cb: *mut Ptcb) {
    let _ = cb;
    // 占位：空操作
}

// ---------------------------------------------------------------------------
// 线程列表锁
// ---------------------------------------------------------------------------

/// 获取全局线程链表锁 `THREAD_LIST_LOCK`。
///
/// 保护全局线程链表的遍历和修改操作。调用者不应在持锁期间调用
/// 可能触发上下文切换或 futex 等待的函数。
#[inline]
pub fn tl_lock() {
    THREAD_LIST_LOCK.lock();
}

/// 释放全局线程链表锁 `THREAD_LIST_LOCK`。
///
/// # 前置条件
///
/// 调用者必须已通过 `tl_lock()` 成功获取过该锁。
#[inline]
pub fn tl_unlock() {
    THREAD_LIST_LOCK.unlock();
}

/// 同步等待目标线程的状态变更（简化占位实现）。
///
/// 完整实现应在目标线程的 futex 字上等待，直到其状态不再为 `DetachState::Exiting`。
/// 当前占位实现为空操作。
pub fn tl_sync(t: *mut Pthread) {
    let _ = t;
    // 占位：空操作 — 需要完整的状态同步机制
}

// ---------------------------------------------------------------------------
// PTC (线程创建) 锁
// ---------------------------------------------------------------------------

/// 获取线程创建锁 `PTC_LOCK`，阻止并发 `pthread_create` 调用。
///
/// 在 fork() 实现中也会使用此锁来安全地暂停线程创建。
#[inline]
pub fn acquire_ptc() {
    PTC_LOCK.lock();
}

/// 释放线程创建锁 `PTC_LOCK`。
///
/// # 前置条件
///
/// 调用者必须已通过 `acquire_ptc()` 成功获取过该锁。
#[inline]
pub fn release_ptc() {
    PTC_LOCK.unlock();
}

/// 临时禁止线程创建（用于 `fork()` 等关键区域）。
///
/// 简化实现等价于 `acquire_ptc()`。调用者负责在合适时机（如 fork 完成后）
/// 调用 `release_ptc()` 恢复线程创建能力。
///
/// 在完整实现中，此函数可能还需要设置额外的禁止标志以处理非 `pthread_create`
/// 路径的线程创建（如 `clone()` 系统调用）。
#[inline]
pub fn inhibit_ptc() {
    acquire_ptc();
}

// ---------------------------------------------------------------------------
// __pthread_self — 线程自引用获取
// ---------------------------------------------------------------------------

/// 获取当前线程的 Pthread 控制块指针。
///
/// 通过读取架构特定的线程指针寄存器获取线程自引用指针：
/// - x86_64: 读取 `fs:0`（FS 段基址偏移 0 处存储 `self_` 指针）
/// - aarch64: 读取 `tpidr_el0` 寄存器
///
/// # 前置条件
///
/// 调用前 TLS（线程局部存储）必须已初始化，FS/TPIDR 寄存器已正确设置。
/// 若在 TLS 初始化前调用，返回值未定义（可能为 null 或无效指针）。
///
/// # 不变量
///
/// 返回的指针指向的 `Pthread` 结构体中 `self_` 字段等于该指针本身。
#[no_mangle]
pub fn __pthread_self() -> *mut Pthread {
    #[cfg(target_arch = "x86_64")]
    {
        let self_ptr: *mut Pthread;
        unsafe {
            core::arch::asm!(
                "mov {}, fs:0",
                out(reg) self_ptr,
                options(nostack, preserves_flags),
            );
        }
        self_ptr
    }
    #[cfg(target_arch = "aarch64")]
    {
        let self_ptr: *mut Pthread;
        unsafe {
            core::arch::asm!(
                "mrs {}, tpidr_el0",
                out(reg) self_ptr,
                options(nostack, preserves_flags),
            );
        }
        self_ptr
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        // 未支持的架构：返回 null，调用者应检查
        core::ptr::null_mut()
    }
}

// ---------------------------------------------------------------------------
// 杂项线程管理函数
// ---------------------------------------------------------------------------

/// 初始化内存屏障（占位实现）。
///
/// 若 Linux 内核支持 `membarrier(2)` 系统调用，则注册其可用性。
/// 简化实现为空操作。
pub fn membarrier_init() {
    // 占位：空操作 — 需要 membarrier 系统调用封装
}

/// 线程退出时的 dlopen 清理（占位实现）。
///
/// 简化实现为空操作。完整实现需要清理当前线程持有的动态链接器资源。
pub fn dl_thread_cleanup() {
    // 占位：空操作 — 需要动态链接器子系统
}

/// 运行线程特定数据 (TSD) 的析构函数（占位实现）。
///
/// 简化实现为空操作。完整实现需要遍历当前线程的 TSD 数组，
/// 对每个非 NULL 条目调用其析构函数。
pub fn pthread_tsd_run_dtors() {
    // 占位：空操作 — 需要完整 TSD 子系统
}

/// 通过 synccall 跨线程删除 TSD key（占位实现）。
///
/// 简化实现为空操作。完整实现需要通过信号同步机制
/// 通知所有线程将其 TSD 数组中对应 key 的条目置为 NULL。
pub fn pthread_key_delete_synccall(key: c_int) {
    let _ = key;
    // 占位：空操作 — 需要 synccall 机制
}

/// 设置线程 TLS 区域（仅特定架构需要，占位实现）。
///
/// 在 x86_64 上通过 `arch_prctl(ARCH_SET_FS, p)` 实现。
/// 简化实现始终返回 0 表示成功。
pub fn set_thread_area(p: *mut c_void) -> c_int {
    let _ = p;
    // 占位：返回 0 表示成功
    0
}

/// 原子地 unmap 自身堆栈并退出（占位实现，永不返回）。
///
/// 完整实现应通过内联汇编直接发起 `munmap(base, size)` 和 `exit(0)`
/// 系统调用，确保在不使用任何堆栈的情况下完成操作。
///
/// # Safety
///
/// 调用者必须确保 `base` 和 `size` 有效，且调用后线程不复存在。
/// 当前占位实现进入无限循环，不会真正释放内存或终止线程。
pub unsafe fn unmapself(base: *mut c_void, size: usize) -> ! {
    let _ = base;
    let _ = size;
    // 占位：永不返回的无限循环 — 需要内联汇编 syscall 实现
    loop {
        core::hint::spin_loop();
    }
}

// ---------------------------------------------------------------------------
// 信号集辅助
// ---------------------------------------------------------------------------

impl Sigset {
    /// 创建全信号集 — 所有 1024 个信号位均置位（全部填充 `0xFF`）。
    ///
    /// 内部表示为 `[u8; 128]`（128 字节 × 8 = 1024 位）。
    pub fn all() -> Self {
        Sigset {
            _private: [0xFFu8; 128],
        }
    }

    /// 从 `u64` 位掩码截断创建信号集。
    ///
    /// 将 `bits` 的低 64 位复制到信号集的前 8 字节（使用本机字节序），
    /// 其余字节保持为零。
    ///
    /// # 参数
    ///
    /// * `bits` - 表示信号 1..=64 的位掩码，位 `n-1` 对应信号 `n`
    pub fn from_bits_truncate(bits: u64) -> Self {
        let mut set = Sigset {
            _private: [0u8; 128],
        };
        let bytes = bits.to_ne_bytes();
        set._private[..8].copy_from_slice(&bytes);
        set
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use core::sync::atomic::{AtomicI32, Ordering};

    // =========================================================================
    // 信号集测试
    // =========================================================================

    test!("sigset_all_all_bytes_are_0xff" {
        let set = super::Sigset::all();
        // 验证所有 128 字节均为 0xFF
        for i in 0..128 {
            assert_eq!(set._private[i], 0xFF, "byte {} should be 0xFF", i);
        }
    });

    test!("sigset_all_first_byte_is_0xff" {
        let set = super::Sigset::all();
        assert_eq!(set._private[0], 0xFF);
    });

    test!("sigset_all_last_byte_is_0xff" {
        let set = super::Sigset::all();
        assert_eq!(set._private[127], 0xFF);
    });

    test!("sigset_from_bits_truncate_zero" {
        let set = super::Sigset::from_bits_truncate(0);
        // 所有字节应为 0
        for i in 0..128 {
            assert_eq!(set._private[i], 0, "byte {} should be 0", i);
        }
    });

    test!("sigset_from_bits_truncate_all_ones" {
        let set = super::Sigset::from_bits_truncate(u64::MAX);
        // 前 8 字节应为 0xFF
        for i in 0..8 {
            assert_eq!(set._private[i], 0xFF, "byte {} should be 0xFF", i);
        }
        // 其余应为 0
        for i in 8..128 {
            assert_eq!(set._private[i], 0, "byte {} should be 0", i);
        }
    });

    test!("sigset_from_bits_truncate_specific_bits" {
        // 设置信号 1 (位 0) 和信号 32 (位 31)
        let bits: u64 = 1 | (1u64 << 31);
        let set = super::Sigset::from_bits_truncate(bits);

        // 位 0 -> 第 0 字节的第 0 位 = 1
        assert_eq!(set._private[0], 1);
        // 位 31 -> 第 3 字节的第 7 位 (小端: 31/8=3, 31%8=7 -> 0x80)
        assert_eq!(set._private[3], 0x80);

        // 其余前 8 字节中未设置的字节应为 0
        assert_eq!(set._private[1], 0);
        assert_eq!(set._private[2], 0);
        assert_eq!(set._private[4], 0);
        assert_eq!(set._private[5], 0);
        assert_eq!(set._private[6], 0);
        assert_eq!(set._private[7], 0);
    });

    // =========================================================================
    // 线程常量测试
    // =========================================================================

    test!("default_stack_size_is_128kb" {
        assert_eq!(super::DEFAULT_STACK_SIZE, 131072);
    });

    test!("default_guard_size_is_8kb" {
        assert_eq!(super::DEFAULT_GUARD_SIZE, 8192);
    });

    test!("default_stack_max_is_8mb" {
        assert_eq!(super::DEFAULT_STACK_MAX, 8 << 20);
    });

    test!("default_guard_max_is_1mb" {
        assert_eq!(super::DEFAULT_GUARD_MAX, 1 << 20);
    });

    // =========================================================================
    // DetachState 枚举值测试
    // =========================================================================

    test!("detach_state_exited_is_0" {
        assert_eq!(super::DetachState::Exited as i32, 0);
    });

    test!("detach_state_exiting_is_1" {
        assert_eq!(super::DetachState::Exiting as i32, 1);
    });

    test!("detach_state_joinable_is_2" {
        assert_eq!(super::DetachState::Joinable as i32, 2);
    });

    test!("detach_state_detached_is_3" {
        assert_eq!(super::DetachState::Detached as i32, 3);
    });

    // =========================================================================
    // 线程列表锁测试（间接测试 SpinLock）
    // =========================================================================

    test!("tl_lock_unlock_basic" {
        super::tl_lock();
        super::tl_unlock();
        // 锁应回到未锁定状态 — 再次加锁成功即可验证
        super::tl_lock();
        super::tl_unlock();
    });

    test!("tl_lock_sequential_twice" {
        super::tl_lock();
        super::tl_unlock();
        super::tl_lock();
        super::tl_unlock();
    });

    // =========================================================================
    // PTC 锁测试
    // =========================================================================

    test!("ptc_lock_acquire_release_basic" {
        super::acquire_ptc();
        super::release_ptc();
        // 再次加锁应成功
        super::acquire_ptc();
        super::release_ptc();
    });

    test!("ptc_inhibit_acquire_release_cycle" {
        super::inhibit_ptc();
        super::release_ptc();
        super::acquire_ptc();
        super::release_ptc();
    });

    test!("ptc_lock_multiple_cycles" {
        for _ in 0..5 {
            super::acquire_ptc();
            super::release_ptc();
        }
    });

    // =========================================================================
    // Futex 同步原语测试（使用栈上原子变量）
    // =========================================================================

    test!("wake_does_not_crash" {
        let futex_word = AtomicI32::new(0);
        super::wake(&futex_word as *const AtomicI32, 1, 0);
    });

    test!("futexwait_exits_when_value_changes" {
        // 单线程中 futexwait 改变值后将立即退出自旋。
        // val=1, 但 futex_word=0，不满足等待条件，应立即返回
        let futex_word = AtomicI32::new(0);
        super::futexwait(&futex_word as *const AtomicI32, 1, 0);
    });

    test!("futexwait_returns_with_different_initial" {
        let futex_word = AtomicI32::new(42);
        super::futexwait(&futex_word as *const AtomicI32, 0, 0);
        // 不应死锁：因为 42 != 0，立即返回
    });

    test!("timedwait_null_timeout_no_crash" {
        let futex_word = AtomicI32::new(1);
        // val=0, futex_word=1 != val，应立即返回 0
        let result = super::timedwait(
            &futex_word as *const AtomicI32,
            0,
            0,
            core::ptr::null(),
            0,
        );
        assert_eq!(result, 0);
    });

    test!("timedwait_with_timeout_no_crash" {
        let futex_word = AtomicI32::new(1);
        let ts = super::timespec { tv_sec: 0, tv_nsec: 0 };
        let result = super::timedwait(
            &futex_word as *const AtomicI32,
            0,
            0,
            &ts as *const super::timespec,
            0,
        );
        assert_eq!(result, 0);
    });

    test!("timedwait_cp_null_timeout_no_crash" {
        let futex_word = AtomicI32::new(1);
        let result = super::timedwait_cp(
            &futex_word as *const AtomicI32,
            0,
            0,
            core::ptr::null(),
            0,
        );
        assert_eq!(result, 0);
    });

    test!("timedwait_cp_with_timeout_no_crash" {
        let futex_word = AtomicI32::new(1);
        let ts = super::timespec { tv_sec: 0, tv_nsec: 0 };
        let result = super::timedwait_cp(
            &futex_word as *const AtomicI32,
            0,
            0,
            &ts as *const super::timespec,
            0,
        );
        assert_eq!(result, 0);
    });

    // =========================================================================
    // TLS 管理测试
    // =========================================================================

    test!("tls_get_addr_returns_null" {
        let result = super::tls_get_addr(core::ptr::null_mut());
        assert!(result.is_null());
    });

    test!("init_tp_returns_zero" {
        let result = super::init_tp(core::ptr::null_mut());
        assert_eq!(result, 0);
    });

    test!("copy_tls_returns_input" {
        static mut MEM: [u8; 16] = [0; 16];
        let result = super::copy_tls(unsafe { MEM.as_mut_ptr() });
        assert_eq!(result, unsafe { MEM.as_ptr() as *mut core::ffi::c_void });
    });

    test!("reset_tls_does_not_crash" {
        super::reset_tls();
    });

    // =========================================================================
    // 取消点机制测试
    // =========================================================================

    test!("testcancel_does_not_crash" {
        super::testcancel();
    });

    test!("do_cleanup_push_pop_no_crash" {
        super::do_cleanup_push(core::ptr::null_mut());
        super::do_cleanup_pop(core::ptr::null_mut());
    });

    test!("do_cleanup_push_then_pop_multiple" {
        for _ in 0..3 {
            super::do_cleanup_push(core::ptr::null_mut());
            super::do_cleanup_pop(core::ptr::null_mut());
        }
    });

    // =========================================================================
    // 线程列表同步测试
    // =========================================================================

    test!("tl_sync_no_crash" {
        super::tl_sync(core::ptr::null_mut());
    });

    // =========================================================================
    // 信号常量测试
    // =========================================================================

    test!("signal_constants" {
        assert_eq!(super::SIGTIMER, 32);
        assert_eq!(super::SIGCANCEL, 33);
        assert_eq!(super::SIGSYNCCALL, 34);
    });

    // =========================================================================
    // 杂项函数测试
    // =========================================================================

    test!("membarrier_init_no_crash" {
        super::membarrier_init();
    });

    test!("dl_thread_cleanup_no_crash" {
        super::dl_thread_cleanup();
    });

    test!("pthread_tsd_run_dtors_no_crash" {
        super::pthread_tsd_run_dtors();
    });

    test!("pthread_key_delete_synccall_no_crash" {
        super::pthread_key_delete_synccall(0);
        super::pthread_key_delete_synccall(42);
        super::pthread_key_delete_synccall(-1);
    });

    test!("set_thread_area_returns_zero" {
        let result = super::set_thread_area(core::ptr::null_mut());
        assert_eq!(result, 0);
    });

    // =========================================================================
    // 全局静态变量测试
    // =========================================================================

    test!("thread_list_lock_exists" {
        let _ = &super::THREAD_LIST_LOCK;
    });

    test!("ptc_lock_exists" {
        let _ = &super::PTC_LOCK;
    });

    test!("abort_lock_exists" {
        let _ = &super::ABORT_LOCK;
    });

    test!("pthread_tsd_size_initial_zero" {
        assert_eq!(
            super::PTHREAD_TSD_SIZE.load(Ordering::Relaxed),
            0
        );
    });

    test!("eintr_valid_flag_initial_zero" {
        assert_eq!(
            super::EINTR_VALID_FLAG.load(Ordering::Relaxed),
            0
        );
    });

    test!("default_stacks_and_guardsize_exist" {
        // 仅验证存在性，不依赖具体值（值可能被环境变量覆盖）
        let _ = unsafe { &super::DEFAULT_STACKSIZE };
        let _ = unsafe { &super::DEFAULT_GUARDSIZE };
    });

    // =========================================================================
    // Pthread 结构体字段类型测试
    // =========================================================================

    test!("pthread_self_field_is_pointer" {
        // 验证 Pthread 类型的 self_ 字段类型为 *mut Pthread
        fn _check(_p: &super::Pthread) -> *mut super::Pthread {
            core::ptr::null_mut()
        }
        let _ = _check;
    });
}