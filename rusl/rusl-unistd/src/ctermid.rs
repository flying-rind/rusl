//! ctermid — 生成控制终端的路径名字符串。
//! 对应 musl src/unistd/ctermid.c
//!
//! 始终返回 "/dev/tty"。

use core::ffi::c_char;

/// POSIX `ctermid` — 返回控制终端的路径名。
///
/// 在 Linux/musl 上始终返回 `"/dev/tty"`。
/// 若 `s` 非空，将字符串拷贝到 `s` 指向的缓冲区并返回 `s`；
/// 若 `s` 为 `NULL`，直接返回指向只读字符串 `"/dev/tty"` 的指针。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn ctermid(s: *mut c_char) -> *mut c_char {
    let path = b"/dev/tty\0";
    if s.is_null() {
        path.as_ptr() as *mut c_char
    } else {
        unsafe {
            let src = path.as_ptr() as *const u8;
            let dst = s as *mut u8;
            let mut i = 0;
            loop {
                let byte = *src.add(i);
                *dst.add(i) = byte;
                if byte == 0 {
                    break;
                }
                i += 1;
            }
        }
        s
    }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::c_char;
    use rusl_core::test;

    test!("test_ctermid_null_pointer_returns_dev_tty" {
        // ctermid(NULL) 返回指向字符串 "/dev/tty" 的指针
        let result = ctermid(core::ptr::null_mut());
        assert!(!result.is_null(), "ctermid(NULL) should not return null");
        unsafe {
            assert_eq!(*result, '/' as c_char, "first char should be '/'");
            let slice = core::slice::from_raw_parts(result as *const u8, 9);
            assert_eq!(slice, b"/dev/tty\0", "should be '/dev/tty' with null terminator");
        }
    });

    test!("test_ctermid_buffer_copies_path" {
        // ctermid(s) 将 "/dev/tty\0" 拷贝到 s, 返回 s
        let mut buf: [c_char; 16] = [1; 16];
        let result = ctermid(buf.as_mut_ptr());
        assert_eq!(result, buf.as_mut_ptr(), "ctermid(s) should return s");
        // 验证拷贝的内容
        let buf_bytes = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, 9) };
        assert_eq!(buf_bytes, b"/dev/tty\0", "buffer should contain '/dev/tty' with null");
    });

    test!("test_ctermid_small_buffer_overflow_check" {
        // 测试小缓冲区: 确保不越界写入
        let mut buf: [c_char; 10] = [0x7F; 10];
        let _result = ctermid(buf.as_mut_ptr());
        // /dev/tty\0 正好是 9 字节, 放在 10 字节缓冲区中刚好
        unsafe {
            assert_eq!(*buf.as_ptr().add(8) as u8, 0, "9th byte should be null terminator");
            // 第 10 个字节不应被修改
            assert_eq!(*buf.as_ptr().add(9) as u8, 0x7F, "10th byte should be untouched");
        }
    });

    test!("test_ctermid_null_and_nonnull_independent" {
        // ctermid(NULL) 和 ctermid(s) 应各自独立工作
        let result1 = ctermid(core::ptr::null_mut());
        let mut buf: [c_char; 16] = [0; 16];
        let result2 = ctermid(buf.as_mut_ptr());
        // 两者都应该返回正确的 "/dev/tty"
        assert!(!result1.is_null());
        assert_eq!(result2, buf.as_mut_ptr());
        let buf_bytes = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, 9) };
        assert_eq!(buf_bytes, b"/dev/tty\0");
    });
}
