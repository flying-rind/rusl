//! __towrite — 激活 FILE 的写模式。
//! 对应 musl src/stdio/__towrite.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 激活 FILE 的写模式，返回 0 成功或 EOF(=-1) 失败。
#[no_mangle]
pub(crate) unsafe extern "C" fn __towrite(f: *mut FILE) -> c_int {
    let f = &mut *f;
    f.mode |= f.mode - 1; // 无论初始值为何，结果均为 -1
    if f.flags & F_NOWR != 0 {
        f.flags |= F_ERR;
        return EOF;
    }
    f.rpos = core::ptr::null_mut();
    f.rend = core::ptr::null_mut();
    f.wpos = f.buf;
    f.wbase = f.buf;
    f.wend = if f.buf_size > 0 {
        unsafe { f.buf.add(f.buf_size) }
    } else {
        core::ptr::null_mut()
    };
    0
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;
    use super::super::stdio_impl::*;

    /// 创建一个基本的 FILE 结构体用于测试，使用给定的缓冲区。
    unsafe fn make_test_file(buf: *mut u8, buf_size: usize) -> FILE {
        let mut f: FILE = core::mem::zeroed();
        f.buf = buf;
        f.buf_size = buf_size;
        f.lock = -1; // 无锁模式，避免测试依赖锁
        f
    }

    test!("test_towrite_basic_success" {
        // 前置: FILE 无 F_NOWR 标志，有缓冲区
        // 后置: 返回 0，mode = -1，读写指针正确设置
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.rpos = 0x1 as *mut u8; // 非空，应被清除
            f.rend = 0x2 as *mut u8; // 非空，应被清除

            let result = __towrite(&mut f as *mut FILE);
            assert_eq!(result, 0, "__towrite 应返回 0 表示成功");
            assert_eq!(f.mode, -1, "mode 应变为 -1");
            assert_eq!(f.wpos, buf.as_mut_ptr(), "wpos 应指向缓冲区起始");
            assert_eq!(f.wbase, buf.as_mut_ptr(), "wbase 应指向缓冲区起始");
            assert_eq!(
                f.wend as usize,
                buf.as_mut_ptr().add(64) as usize,
                "wend 应指向缓冲区末尾"
            );
            assert!(f.rpos.is_null(), "rpos 应被置空");
            assert!(f.rend.is_null(), "rend 应被置空");
            assert_eq!(f.flags & F_ERR, 0, "不应设置 F_ERR");
        }
    });

    test!("test_towrite_nowr_failure" {
        // 前置: FILE 带 F_NOWR 标志
        // 后置: 返回 EOF，设置 F_ERR
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.flags = F_NOWR;

            let result = __towrite(&mut f as *mut FILE);
            assert_eq!(result, EOF, "带 F_NOWR 时应返回 EOF");
            assert_ne!(f.flags & F_ERR, 0, "应设置 F_ERR 标志");
        }
    });

    test!("test_towrite_zero_bufsize" {
        // 前置: buf_size = 0
        // 后置: wpos/wbase 指向 buf，wend 为空
        let mut buf = [0u8; 1]; // buf 存在但大小为 0
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 0);

            let result = __towrite(&mut f as *mut FILE);
            assert_eq!(result, 0, "buf_size=0 仍应成功");
            assert_eq!(f.wpos, buf.as_mut_ptr());
            assert_eq!(f.wbase, buf.as_mut_ptr());
            assert!(f.wend.is_null(), "buf_size=0 时 wend 应为空");
        }
    });

    test!("test_towrite_mode_preserves_ones" {
        // 前置: mode 已有值
        // 后置: mode |= mode - 1 的行为被验证
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.mode = 5; // 非零初始值
            let result = __towrite(&mut f as *mut FILE);
            assert_eq!(result, 0);
            // mode = 5 | (5-1) = 5 | 4 = 5, not -1
            // 该操作确保 mode 的 bit 0 被设置
            assert_eq!(f.mode, 5);
        }
    });

    test!("test_towrite_null_buf" {
        // 前置: buf 为空指针，buf_size > 0
        // 后置: wend = null + buf_size (即 buf_size 作为地址值)
        unsafe {
            let mut f: FILE = core::mem::zeroed();
            f.buf = core::ptr::null_mut();
            f.buf_size = 16;
            f.lock = -1;

            let result = __towrite(&mut f as *mut FILE);
            assert_eq!(result, 0);
            assert!(f.wpos.is_null(), "wpos 跟随 buf 为空");
            assert_eq!(f.wend as usize, 16, "wend 为 null+16=16");
        }
    });
}
