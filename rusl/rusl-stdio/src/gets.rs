//! gets — 从 stdin 读取一行到用户缓冲区（无边界检查）。
//! 对应 musl src/stdio/gets.c
//!
//! 严重安全警告：此函数不对缓冲区进行边界检查。C11 已移除。
//! 仅出于 ABI 兼容性保留。新代码应使用 fgets。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// 从标准输入 stdin 读取字符直到 '\n' 或 EOF，存入缓冲区 s（不含 '\n'，以 '\0' 结尾）。
/// 无缓冲区边界检查 —— 本质不安全。
/// [Visibility]: User — C89 标准库函数（C11 已移除，POSIX.1-2008 标记为过时）。
#[no_mangle]
pub extern "C" fn gets(s: *mut c_char) -> *mut c_char {
    unsafe {
        let mut i: usize = 0;
        let stdin_ptr = super::stdin::stdin as *mut super::stdio_impl::FILE;
        let mut c: i32;

        loop {
            c = super::getc_unlocked::getc_unlocked(stdin_ptr);
            if c == super::stdio_impl::EOF || c == b'\n' as i32 {
                break;
            }
            *s.add(i) = c as c_char;
            i += 1;
        }
        *s.add(i) = 0;

        // musl: if (c != '\n' && (!feof(stdin) || !i)) s = 0;
        if c != b'\n' as i32 {
            let stdin_ref = &*stdin_ptr;
            let is_eof = stdin_ref.flags & super::stdio_impl::F_EOF != 0;
            if !is_eof || i == 0 {
                return core::ptr::null_mut();
            }
        }

        s
    }
}
