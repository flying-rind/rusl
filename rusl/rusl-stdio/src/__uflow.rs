//! __uflow — 从 FILE 流读取一个字符。
//! 对应 musl src/stdio/__uflow.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 从 FILE 流获取下一个字符（仅在缓冲区为空时调用）。
#[no_mangle]
pub(crate) unsafe extern "C" fn __uflow(f: *mut FILE) -> c_int {
    if super::__toread::__toread(f) != 0 {
        return EOF;
    }
    if let Some(read_fn) = (*f).read {
        let mut c: u8 = 0;
        if read_fn(f, &raw mut c, 1) == 1 {
            return c as c_int;
        }
    }
    EOF
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;
    use super::super::stdio_impl::*;

    /// 创建测试用 FILE，可指定是否带 F_NORD（导致 __toread 失败）
    unsafe fn make_test_file(buf: *mut u8, buf_size: usize, nord: bool) -> FILE {
        let mut f: FILE = core::mem::zeroed();
        f.buf = buf;
        f.buf_size = buf_size;
        f.lock = -1;
        if nord {
            f.flags = F_NORD;
        }
        f
    }

    test!("test_uflow_toread_fails" {
        // 前置: FILE 带 F_NORD 标志（__toread 将失败）
        // 后置: __uflow 返回 EOF
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64, true);

            let result = __uflow(&mut f as *mut FILE);
            assert_eq!(result, EOF, "__toread 失败时 __uflow 应返回 EOF");
        }
    });

    test!("test_uflow_no_read_callback" {
        // 前置: __toread 成功但 read 回调为 None
        // 后置: __uflow 返回 EOF
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64, false);
            f.read = None;

            let result = __uflow(&mut f as *mut FILE);
            assert_eq!(result, EOF, "无 read 回调时应返回 EOF");
        }
    });

    test!("test_uflow_read_success" {
        // 前置: __toread 成功，read 回调返回 1 字节
        // 后置: __uflow 返回该字节值
        unsafe extern "C" fn mock_read(_f: *mut FILE, buf: *mut u8, _len: usize) -> usize {
            if !buf.is_null() {
                *buf = b'X';
                1
            } else {
                0
            }
        }

        let mut file_buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(file_buf.as_mut_ptr(), 64, false);
            f.read = Some(mock_read);

            let result = __uflow(&mut f as *mut FILE);
            assert_eq!(result, b'X' as i32, "应返回读取到的字符 'X'");
        }
    });

    test!("test_uflow_read_returns_zero" {
        // 前置: __toread 成功，read 回调返回 0（无数据）
        // 后置: __uflow 返回 EOF
        unsafe extern "C" fn empty_read(_f: *mut FILE, _buf: *mut u8, _len: usize) -> usize {
            0 // 模拟 EOF 情况
        }

        let mut file_buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(file_buf.as_mut_ptr(), 64, false);
            f.read = Some(empty_read);

            let result = __uflow(&mut f as *mut FILE);
            assert_eq!(result, EOF, "read 返回 0 时 __uflow 应返回 EOF");
        }
    });

    test!("test_uflow_non_ascii_byte" {
        // 前置: __toread 成功，read 返回非 ASCII 字节 (0x80-0xFF)
        // 后置: __uflow 返回该字节（不应扩展符号，应为正值）
        unsafe extern "C" fn nonascii_read(_f: *mut FILE, buf: *mut u8, _len: usize) -> usize {
            *buf = 0xE9; // 特殊字符，高位字节
            1
        }

        let mut file_buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(file_buf.as_mut_ptr(), 64, false);
            f.read = Some(nonascii_read);

            let result = __uflow(&mut f as *mut FILE);
            // c_int 是 i32，u8->i32 应保持正值
            assert_eq!(result, 0xE9, "非 ASCII 字节应被正确返回");
            assert!(result > 0, "返回值应为正值");
        }
    });
}
