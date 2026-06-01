//! `__libc_start_main` — C 运行时入口点。
//!
//! 对应 musl `src/env/__libc_start_main.c`。
//!
//! 实现从 ELF 入口 `_start` 到用户 `main()` 之间的所有初始化与过渡工作。
//!
//! # 启动链（两阶段设计）
//!
//! ```text
//! _start (crt1.S)
//!   → _start_c
//!     → __libc_start_main              // Stage 1: 调用 __init_libc
//!       → __init_libc                  // 初始化 environ, auxv, TLS, SSP, SUID/SGID
//!       → compiler_fence               // 屏障：防止 TLS/SSP 访问被提升
//!       → libc_start_main_stage2       // Stage 2: 新栈帧（释放 stage1 栈空间）
//!         → libc_start_init            // _init() + .init_array 构造器
//!         → main(argc, argv, envp)     // 用户程序入口
//!         → exit(ret)                  // 永不返回
//! ```

#![allow(bad_asm_style)] // .intel_syntax / .att_syntax in global_asm!

use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::Ordering;

use rusl_core::c_types::size_t;
use rusl_core::syscall::{raw_syscall1, raw_syscall3, raw_syscall5};
use rusl_core::syscall::{SYS_exit, SYS_exit_group, SYS_open};

// ============================================================================
// _start — per-arch entry point (global_asm!)
// ============================================================================

unsafe extern "C" {
    pub unsafe fn _start(argc: i32, argv: *const *const u8) -> i32;
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".section .text._start,\"ax\",@progbits",
    ".global _start",
    ".type _start,@function",
    ".intel_syntax noprefix",
    "_start:",
    "xor ebp, ebp",         // zero frame pointer — marks outermost frame
    "mov rdi, rsp",         // arg1 = stack pointer (→ _start_c)
    "and rsp, -16",         // 16-byte align for SSE ABI
    "call _start_c",        // never returns
    ".att_syntax prefix",
    ".size _start, .-_start",
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".section .text._start,\"ax\",%progbits",
    ".global _start",
    ".type _start,%function",
    "_start:",
    "mov x29, #0",          // zero frame pointer
    "mov x0, sp",           // arg1 = stack pointer (→ _start_c)
    "and sp, sp, #-16",     // 16-byte align
    "b _start_c",           // tail call (never returns)
    ".size _start, .-_start",
);

// ============================================================================
// 常量定义
// ============================================================================

/// 辅助向量本地缓存数组长度（必须 >= 内核可能传递的所有 AT_* 类型最大索引值）
const AUX_CNT: usize = 38;

// AT_* 辅助向量类型常量
const AT_HWCAP:   usize = 16;
const AT_SYSINFO: usize = 32;
const AT_PAGESZ:  usize = 6;
const AT_EXECFN:  usize = 31;
const AT_RANDOM:  usize = 25;
const AT_UID:     usize = 11;
const AT_EUID:    usize = 12;
const AT_GID:     usize = 13;
const AT_EGID:    usize = 14;
const AT_SECURE:  usize = 23;

// poll / open 常量
const POLLNVAL: i16 = 0x020;
const O_RDWR:   i64 = 2;

// ============================================================================
// 链接器定义的符号 — .init_array 段起止地址
// ============================================================================

extern "C" {
    #[link_name = "__init_array_start"]
    static __init_array_start: unsafe extern "C" fn();
    #[link_name = "__init_array_end"]
    static __init_array_end: unsafe extern "C" fn();
}

// ============================================================================
// InitHooks — 替代 C weak_alias 的函数指针间接层
// ============================================================================

/// 初始化钩子表：替代 C 的 weak_alias 间接性。
///
/// 默认函数指针均指向空操作 dummy 实现。
/// 外部模块可在链接阶段通过初始化顺序约定替换这些指针。
pub(crate) struct InitHooks {
    /// 对应 C: weak_alias(dummy, _init)
    pub(crate) init_fn:     extern "C" fn(),
    /// 对应 C: weak_alias(dummy1, __init_ssp)
    pub(crate) init_ssp_fn: extern "C" fn(*mut c_void),
}

extern "C" fn dummy() {}

extern "C" fn dummy1(_p: *mut c_void) {}

pub(crate) static mut INIT_HOOKS: InitHooks = InitHooks {
    init_fn: dummy,
    init_ssp_fn: dummy1,
};

// ============================================================================
// _init — System V ABI 兼容的弱符号桩
// ============================================================================

/// 默认委托给 `INIT_HOOKS.init_fn` (即 `dummy`)。
/// 用户可通过链接时提供同名符号覆盖默认行为。
#[no_mangle]
#[linkage = "weak"]
pub unsafe extern "C" fn _init() {
    (INIT_HOOKS.init_fn)();
}

// ============================================================================
// _start_c — C entry point
// ============================================================================

/// 用户 main 函数类型。
type MainFn = unsafe extern "C" fn(c_int, *const *const c_char, *const *const c_char) -> c_int;

extern "C" {
    fn main(argc: c_int, argv: *const *const c_char, envp: *const *const c_char) -> c_int;
}

/// 由 _start 汇编调用，传递 rsp (栈指针) 作为参数。
#[no_mangle]
pub unsafe extern "C" fn _start_c(sp: *const i64) -> ! {
    let argc = *sp as c_int;
    let argv = sp.add(1) as *const *const c_char;
    let _ = __libc_start_main(
        main,
        argc,
        argv,
        core::ptr::null(),
        core::ptr::null(),
        core::ptr::null(),
    );
    // 不可达：libc_start_main_stage2 → exit → _Exit 永不返回
    rusl_internal::atomic::a_crash();
}

// ============================================================================
// __libc_start_main — libc startup orchestrator (stage 1)
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn __libc_start_main(
    main_fn: MainFn,
    argc: c_int,
    argv: *const *const c_char,
    _init_dummy: *const c_void,
    _fini_dummy: *const c_void,
    _ldso_dummy: *const c_void,
) -> c_int {
    let envp = argv.add(argc as usize + 1);

    // Stage 1: libc 初始化。
    // __init_libc 标注为 #[inline(never)]，其栈帧在调用返回后被释放。
    __init_libc(envp as *mut *mut c_char, *argv);

    // 编译器屏障：防止应用代码、SSP 访问、线程指针访问
    // 在 TLS/SSP 初始化之前被编译器提升。
    // 对应 C: __asm__("" : "+r"(stage2) : : "memory");
    core::sync::atomic::compiler_fence(Ordering::SeqCst);

    // Stage 2: 通过函数调用"返回"到 stage2。
    // stage2 获得新的栈帧，释放第一阶段的栈空间。
    libc_start_main_stage2(main_fn, argc, argv)
}

// ============================================================================
// __init_libc — libc 初始化中枢
// ============================================================================

/// libc 初始化中枢，负责从内核辅助向量中提取系统参数、初始化全局状态、
/// 设置 TLS/SSP，并检测 SUID/SGID 安全执行模式。
///
/// 本函数在进程生命周期中恰好被调用一次（由 `__libc_start_main` 或
/// 动态链接器的 `_dlstart` 调用）。
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __init_libc(envp: *mut *mut c_char, pn: *const c_char) {
    // ---- 1. 设置 environ ----
    crate::__environ::environ = envp;

    // ---- 2. 定位 auxv ----
    let mut i: isize = 0;
    if !envp.is_null() {
        while !(*envp.offset(i)).is_null() {
            i += 1;
        }
        i += 1; // 跳过 envp 的 NULL 终止符
    }
    let auxv = envp.offset(i) as *mut size_t;
    rusl_internal::libc::__libc.auxv = auxv;

    // ---- 3. 解析 auxv 到本地数组 ----
    let mut aux: [usize; AUX_CNT] = [0; AUX_CNT];
    let mut j: isize = 0;
    loop {
        let atype = *auxv.offset(j);
        if atype == 0 {
            break;
        }
        if atype < AUX_CNT {
            aux[atype] = *auxv.offset(j + 1);
        }
        j += 2;
    }

    // ---- 4. 提取全局系统参数 ----
    rusl_internal::libc::__hwcap = aux[AT_HWCAP];
    if aux[AT_SYSINFO] != 0 {
        rusl_internal::defsysinfo::__SYSINFO.store(aux[AT_SYSINFO], Ordering::Release);
    }
    rusl_internal::libc::__libc.page_size = aux[AT_PAGESZ];

    // ---- 5. 设置程序名 ----
    let mut progname = pn;
    if progname.is_null() {
        progname = aux[AT_EXECFN] as *const c_char;
    }
    if progname.is_null() {
        progname = c"".as_ptr();
    }
    rusl_internal::libc::__progname_full = progname;
    rusl_internal::libc::__progname = progname;
    // 查找 basename: 从后向前扫描最后一个 '/'
    let mut s: isize = 0;
    while *progname.offset(s) != 0 {
        if *progname.offset(s) as u8 == b'/' {
            rusl_internal::libc::__progname = progname.offset(s + 1);
        }
        s += 1;
    }

    // ---- 6. TLS 初始化 ----
    crate::__init_tls::init_tls(aux.as_mut_ptr());

    // ---- 7. SSP 初始化 ----
    // 直接调用 __init_ssp（单 crate 编译，SSP 模块始终存在）。
    // 若 AT_RANDOM 非空，从内核熵源设置栈 canary；否则回退到基于地址的确定性算法。
    crate::__stack_chk_fail::__init_ssp(aux[AT_RANDOM] as *mut c_void);

    // ---- 8. SUID/SGID 安全检测 ----
    // 若非 SUID/SGID 且内核未标记 AT_SECURE，直接返回。
    if aux[AT_UID] == aux[AT_EUID] && aux[AT_GID] == aux[AT_EGID] && aux[AT_SECURE] == 0 {
        return;
    }

    // SUID/SGID 安全模式：验证并修复 fd 0/1/2。
    // 若有 fd 无效 (POLLNVAL)，打开 /dev/null 填充之。
    #[repr(C)]
    struct PollFd {
        fd: c_int,
        events: i16,
        revents: i16,
    }

    let mut pfd: [PollFd; 3] = [
        PollFd { fd: 0, events: 0, revents: 0 },
        PollFd { fd: 1, events: 0, revents: 0 },
        PollFd { fd: 2, events: 0, revents: 0 },
    ];

    // poll(2) 或 ppoll(2) — 取决于架构是否有 SYS_poll
    #[cfg(target_arch = "x86_64")]
    let r = {
        use rusl_core::syscall::SYS_poll;
        raw_syscall3(SYS_poll, pfd.as_mut_ptr() as i64, 3, 0)
    };

    #[cfg(target_arch = "aarch64")]
    let r = {
        // aarch64 无 SYS_poll，使用 SYS_ppoll
        use rusl_core::syscall::SYS_ppoll;
        #[repr(C)]
        struct Timespec { tv_sec: i64, tv_nsec: i64 }
        let ts = Timespec { tv_sec: 0, tv_nsec: 0 };
        raw_syscall5(
            SYS_ppoll,
            pfd.as_mut_ptr() as i64,
            3,
            &ts as *const Timespec as i64,
            0,  // sigmask = NULL
            0,  // sigsetsize (忽略因为 sigmask 为 NULL)
        )
    };

    if r < 0 {
        rusl_internal::atomic::a_crash();
    }

    let dev_null = c"/dev/null".as_ptr();
    for i in 0..3 {
        if pfd[i].revents & POLLNVAL != 0 {
            // open /dev/null with O_RDWR — 返回最低可用 fd
            if raw_syscall3(SYS_open, dev_null as i64, O_RDWR, 0) < 0 {
                rusl_internal::atomic::a_crash();
            }
        }
    }

    rusl_internal::libc::__libc.secure = 1;
}

// ============================================================================
// libc_start_init — 用户初始化函数（模块私有）
// ============================================================================

/// 调用用户定义的初始化函数：
/// 1. `INIT_HOOKS.init_fn`（即 `_init`）
/// 2. `.init_array` 段中的所有构造函数
unsafe fn libc_start_init() {
    // Step 1: legacy _init
    (INIT_HOOKS.init_fn)();

    // Step 2: .init_array 构造器遍历
    let start = &raw const __init_array_start as *const unsafe extern "C" fn();
    let end   = &raw const __init_array_end   as *const unsafe extern "C" fn();
    let mut ptr = start;
    while ptr < end {
        unsafe { (*ptr)(); }
        ptr = ptr.add(1);
    }
}

// ============================================================================
// libc_start_main_stage2 — 第二阶段启动（模块私有）
// ============================================================================

/// 启动流程的第二阶段（也是最后一阶段）。
///
/// 与第一阶段分离为独立函数以释放 `__init_libc` 的栈帧。
/// 返回类型 `!` 强调永不返回（通过 `exit()` 终止）。
unsafe fn libc_start_main_stage2(
    main_fn: MainFn,
    argc: c_int,
    argv: *const *const c_char,
) -> ! {
    let envp = unsafe { argv.add(argc as usize + 1) };
    libc_start_init();
    let ret = main_fn(argc, argv, envp);
    exit(ret);
}

// ============================================================================
// exit / _Exit / _exit — 进程终止
// ============================================================================

/// POSIX _Exit — 立即终止进程，不执行任何清理。
#[no_mangle]
pub unsafe extern "C" fn _Exit(code: c_int) -> ! {
    let c = code as i64;
    // exit_group 终止线程组中所有线程
    raw_syscall1(SYS_exit_group, c);
    // 回退：exit 仅终止当前线程
    raw_syscall1(SYS_exit, c);
    loop {
        core::hint::spin_loop();
    }
}

/// ISO C exit — 调用 atexit 处理函数后终止进程。
/// 当前为最小实现：直接委托给 _Exit（atexit 尚未支持）。
#[no_mangle]
pub unsafe extern "C" fn exit(code: c_int) -> ! {
    _Exit(code)
}

/// POSIX _exit — _Exit 的同义词。
#[no_mangle]
pub unsafe extern "C" fn _exit(code: c_int) -> ! {
    _Exit(code)
}
