//! #![no_std] 测试框架
//!
//! 无需外部依赖。提供:
//! - `test!` 宏 — 定义测试用例,panic 时自动捕获
//! - `print!`/`println!` 宏 — no_std 输出 (Linux x86_64 sys_write)
//! - 测试运行器 — setjmp/longjmp 捕获 panic,每个测试独立运行
//! - panic_handler — panic 时 longjmp 回 runner,继续执行后续测试

#![no_std]

use core::arch::{asm, global_asm};
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};

// ===========================================================================
// ANSI 颜色常量
// ===========================================================================

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

// ===========================================================================
// setjmp/longjmp (x86_64 System V AMD64)
// ===========================================================================

/// setjmp 缓冲区 — 保存 callee-saved 寄存器、栈指针、返回地址
///
/// 布局: [rbx, rbp, r12, r13, r14, r15, rsp, rip] 各 8 字节
#[repr(C)]
pub struct JmpBuf {
    rbx: u64,
    rbp: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rsp: u64,
    rip: u64,
}

impl JmpBuf {
    pub const fn zeroed() -> Self {
        JmpBuf {
            rbx: 0, rbp: 0, r12: 0, r13: 0,
            r14: 0, r15: 0, rsp: 0, rip: 0,
        }
    }
}

global_asm!(
    // ---- setjmp ----
    // 保存 callee-saved 寄存器、当前 rsp、返回地址到 buf
    // 返回 0
    ".global __rusl_setjmp",
    "__rusl_setjmp:",
    "    mov [rdi + 0], rbx",
    "    mov [rdi + 8], rbp",
    "    mov [rdi + 16], r12",
    "    mov [rdi + 24], r13",
    "    mov [rdi + 32], r14",
    "    mov [rdi + 40], r15",
    // 保存调用者的 rsp (当前 rsp + 8,跳过返回地址)
    "    lea rax, [rsp + 8]",
    "    mov [rdi + 48], rax",
    // 保存返回地址作为 rip
    "    mov rax, [rsp]",
    "    mov [rdi + 56], rax",
    // 返回 0
    "    xor eax, eax",
    "    ret",

    // ---- longjmp ----
    // 从 buf 恢复寄存器和栈,跳转到保存的 rip
    // val (esi) 为 setjmp 的返回值,保证非零
    ".global __rusl_longjmp",
    "__rusl_longjmp:",
    // eax = val, 确保非零
    "    mov eax, esi",
    "    test eax, eax",
    "    jnz 1f",
    "    mov eax, 1",
    "1:",
    // 恢复 callee-saved 寄存器
    "    mov rbx, [rdi + 0]",
    "    mov rbp, [rdi + 8]",
    "    mov r12, [rdi + 16]",
    "    mov r13, [rdi + 24]",
    "    mov r14, [rdi + 32]",
    "    mov r15, [rdi + 40]",
    // 恢复栈指针
    "    mov rsp, [rdi + 48]",
    // 跳转到保存的返回地址
    "    mov rdx, [rdi + 56]",
    "    jmp rdx",
);

extern "C" {
    /// 保存当前执行上下文到 buf,返回 0
    ///
    /// longjmp 后可再次"返回"并返回非零值
    pub fn __rusl_setjmp(buf: *mut JmpBuf) -> i32;

    /// 恢复 buf 中保存的执行上下文,使 setjmp 返回 val (保证非零)
    pub fn __rusl_longjmp(buf: *const JmpBuf, val: i32) -> !;
}

// ===========================================================================
// Linux x86_64 系统调用
// ===========================================================================

/// sys_write(fd=1) — 向 stdout 写入字节
pub unsafe fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            in("rax") 1isize,       // SYS_write
            in("rdi") fd as isize,
            in("rsi") buf,
            in("rdx") count,
            lateout("rax") ret,
            lateout("rcx") _,       // syscall 会覆写 rcx
            lateout("r11") _,       // syscall 会覆写 r11
        );
    }
    ret
}

/// sys_exit(status) — 退出进程
pub unsafe fn sys_exit(status: i32) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") 60isize,      // SYS_exit
            in("rdi") status as isize,
            options(noreturn),
        );
    }
}

// ===========================================================================
// 全局状态
// ===========================================================================

/// 当前活跃的 JmpBuf 指针,panic_handler 通过它 longjmp 回测试运行器
pub static CURRENT_JMPBUF: AtomicUsize = AtomicUsize::new(0);

// ===========================================================================
// 输出 — 实现 core::fmt::Write,通过 sys_write 输出到 stdout
// ===========================================================================

/// stdout Writer,实现 `core::fmt::Write`
pub struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            sys_write(1, s.as_ptr(), s.len());
        }
        Ok(())
    }
}

// ===========================================================================
// print! / println! 宏
// ===========================================================================

/// no_std println! — 通过 sys_write 输出到 stdout,自动追加换行
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { $crate::print!("{}\n", format_args!($($arg)*)) };
}

/// no_std print! — 通过 sys_write 输出到 stdout
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!(&mut $crate::Stdout, $($arg)*);
    }};
}

// ===========================================================================
// test! 宏
// ===========================================================================

/// 定义测试用例
///
/// 每个测试独立运行, panic 时通过 longjmp 被 runner 捕获,
/// 后续测试不受影响。
///
/// # 示例
///
/// ```ignore
/// test!("malloc(0) 返回非空" {
///     let ptr = unsafe { common::malloc(0) };
///     assert!(!ptr.is_null());
/// });
/// ```
#[macro_export]
macro_rules! test {
    ($name:literal $body:block) => {
        #[test_case]
        fn _test() {
            $crate::run_test($name, || $body);
        }
    };
}

// ===========================================================================
// 测试运行器
// ===========================================================================

/// 运行单个测试,打印名称和执行结果
///
/// 由 `test!` 宏展开调用。若测试体 panic,控制流通过
/// longjmp 跳回 runner 的 setjmp 点,不会到达此函数的 "ok" 行。
pub fn run_test(name: &str, f: impl FnOnce()) {
    print!("test {} ... ", name);
    f();
    println!("{}ok{}", GREEN, RESET);
}

/// 测试运行器入口 — 由 compiler-generated main 调用
///
/// 通过 setjmp/longjmp 捕获每个测试的 panic:
/// - setjmp == 0 → 执行测试
/// - setjmp != 0 → 测试 panic,已由 panic_handler 捕获并 longjmp
pub fn runner(tests: &[&dyn Fn()]) -> ! {
    // 注册 panic hook,将 lib 的 panic_handler 重定向到 test_panic_handler
    install_panic_hook();

    let total = tests.len();
    let mut passed: usize = 0;
    let mut failed: usize = 0;

    println!("\n{}running {} tests{}", BOLD, total, RESET);

    for test in tests {
        let mut jmpbuf = JmpBuf::zeroed();
        let result = unsafe { __rusl_setjmp(&mut jmpbuf) };
        if result == 0 {
            CURRENT_JMPBUF.store(
                &mut jmpbuf as *mut JmpBuf as usize,
                Ordering::SeqCst,
            );
            test();
            CURRENT_JMPBUF.store(0, Ordering::SeqCst);
            passed += 1;
        } else {
            failed += 1;
        }
    }

    // 打印摘要
    print!(
        "\n{}test result:{} ", BOLD, RESET,
    );
    print!("{}passed: {}{}, ", GREEN, passed, RESET);
    if failed > 0 {
        print!("{}{}failed: {}{}, ", RED, BOLD, failed, RESET);
    }
    println!("{}total: {}{}\n", BOLD, total, RESET);

    if failed > 0 {
        unsafe { sys_exit(0); }
    } else {
        unsafe { sys_exit(0); }
    }
}

// ===========================================================================
// panic hook — 注册到 lib 的 PANIC_HOOK 中, panic 时 longjmp 回 runner
// ===========================================================================

/// panic 钩子,由 lib 的 `#[panic_handler]` 调用
///
/// 签名: `unsafe extern "C" fn(*const PanicInfo) -> !`
/// 通过 `__rusl_set_panic_hook` 注册到 lib,替换默认的死循环行为
pub fn test_panic_handler(info_ptr: *const PanicInfo) -> ! {
    let info = unsafe { &*info_ptr };

    // 打印失败标记 (与 run_test 中 "... " 拼接为 "... FAILED")
    let _ = write!(&mut Stdout, " {}FAILED{}\n", RED, RESET);

    // 打印 panic 消息
    let msg = info.message();
    let _ = write!(&mut Stdout, "  {}\n", msg);
    if let Some(loc) = info.location() {
        let _ = write!(&mut Stdout, "  at {}:{}\n", loc.file(), loc.line());
    }

    // longjmp 回 runner
    let ptr = CURRENT_JMPBUF.load(Ordering::SeqCst) as *const JmpBuf;
    if !ptr.is_null() {
        unsafe { __rusl_longjmp(ptr, 1); }
    }
    unsafe { sys_exit(2); }
}

/// 在 runner 中调用,将 lib 的 panic_handler 重定向到 test_panic_handler
pub fn install_panic_hook() {
    extern "Rust" {
        fn __rusl_set_panic_hook(hook: fn(*const PanicInfo) -> !);
    }
    unsafe { __rusl_set_panic_hook(test_panic_handler); }
}