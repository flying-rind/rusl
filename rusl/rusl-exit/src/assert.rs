//! __assert_fail — 断言失败处理。
//! 对应 musl `src/exit/assert.c`。

use core::ffi::c_int;
use rusl_syscall::__syscall3;
use super::sys_consts::{SYS_write, STDERR_FILENO};

/// 断言失败时打印消息到 stderr 并调用 abort()。
#[no_mangle]
pub unsafe extern "C" fn __assert_fail(
    expr: *const u8,
    file: *const u8,
    line: c_int,
    func: *const u8,
) -> ! {
    // 输出 "Assertion failed: <expr> (<file>: <func>: <line>)\n"
    // 简化版: 不格式化 line 数字 (no_std 无 sprintf)
    let prefix = b"Assertion failed: ";
    unsafe { write_stderr(prefix); }
    unsafe { write_stderr_cstr(expr); }

    let sep1 = b" (";
    unsafe { write_stderr(sep1); }
    unsafe { write_stderr_cstr(file); }

    let sep2 = b": ";
    unsafe { write_stderr(sep2); }
    unsafe { write_stderr_cstr(func); }

    let sep3 = b": ";
    unsafe { write_stderr(sep3); }
    // 输出行号 (简单整数→ASCII)
    write_int_stderr(line);

    let newline = b")\n";
    unsafe { write_stderr(newline); }

    super::abort::abort()
}

unsafe fn write_stderr(buf: &[u8]) {
    unsafe {
        __syscall3(SYS_write, STDERR_FILENO, buf.as_ptr() as i64, buf.len() as i64);
    }
}

unsafe fn write_stderr_cstr(s: *const u8) {
    if s.is_null() {
        return;
    }
    let mut len: usize = 0;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    if len > 0 {
        unsafe {
            __syscall3(SYS_write, STDERR_FILENO, s as i64, len as i64);
        }
    }
}

fn write_int_stderr(n: c_int) {
    if n == 0 {
        unsafe { write_stderr(b"0"); }
        return;
    }
    let mut buf = [0u8; 16];
    let mut i = buf.len();
    let mut v = if n < 0 { -n } else { n };
    while v > 0 {
        i -= 1;
        buf[i] = (v % 10) as u8 + b'0';
        v /= 10;
    }
    if n < 0 {
        i -= 1;
        buf[i] = b'-';
    }
    unsafe { write_stderr(&buf[i..]); }
}
