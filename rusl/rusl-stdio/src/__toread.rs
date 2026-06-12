//! __toread — 激活 FILE 的读模式。
//! 对应 musl src/stdio/__toread.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 激活 FILE 的读模式，返回 0 成功或 EOF(=-1) 失败。
#[no_mangle]
pub(crate) unsafe extern "C" fn __toread(f: *mut FILE) -> c_int {
    let f = &mut *f;
    f.mode |= f.mode - 1;
    if f.wpos != f.wbase {
        if let Some(write_fn) = f.write {
            write_fn(f, core::ptr::null(), 0);
        }
    }
    f.wpos = core::ptr::null_mut();
    f.wbase = core::ptr::null_mut();
    f.wend = core::ptr::null_mut();
    if f.flags & F_NORD != 0 {
        f.flags |= F_ERR;
        return EOF;
    }
    f.rpos = unsafe { f.buf.add(f.buf_size) };
    f.rend = f.rpos;
    if f.flags & F_EOF != 0 { EOF } else { 0 }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;
    use super::super::stdio_impl::*;
    use core::sync::atomic::{AtomicBool, Ordering};

    unsafe fn make_test_file(buf: *mut u8, buf_size: usize) -> FILE {
        let mut f: FILE = core::mem::zeroed();
        f.buf = buf;
        f.buf_size = buf_size;
        f.lock = -1;
        f
    }

    test!("test_toread_basic_success" {
        // 前置: FILE 无 F_NORD/F_EOF，有缓冲区
        // 后置: 返回 0，rpos/rend 正确设置，写指针清空
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);

            let result = __toread(&mut f as *mut FILE);
            assert_eq!(result, 0, "__toread 应返回 0 表示成功");
            assert_eq!(f.mode, -1, "mode 应变为 -1");
            let buf_end = buf.as_mut_ptr().add(64);
            assert_eq!(f.rpos, buf_end, "rpos 应指向缓冲区末尾（空缓冲区）");
            assert_eq!(f.rend, buf_end, "rend 应指向缓冲区末尾（空缓冲区）");
            assert!(f.wpos.is_null(), "wpos 应被置空");
            assert!(f.wbase.is_null(), "wbase 应被置空");
            assert!(f.wend.is_null(), "wend 应被置空");
        }
    });

    test!("test_toread_nord_failure" {
        // 前置: FILE 带 F_NORD 标志
        // 后置: 返回 EOF，设置 F_ERR
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.flags = F_NORD;

            let result = __toread(&mut f as *mut FILE);
            assert_eq!(result, EOF, "带 F_NORD 时应返回 EOF");
            assert_ne!(f.flags & F_ERR, 0, "应设置 F_ERR 标志");
        }
    });

    test!("test_toread_eof_already_set" {
        // 前置: FILE 带 F_EOF 标志
        // 后置: 返回 EOF（即使无 F_NORD）
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.flags = F_EOF;

            let result = __toread(&mut f as *mut FILE);
            assert_eq!(result, EOF, "带 F_EOF 时应返回 EOF");
            assert_eq!(f.rpos, buf.as_mut_ptr().add(64), "rpos 应指向缓冲区末尾");
        }
    });

    test!("test_toread_pending_write_flushed" {
        // 前置: wpos != wbase（有未刷新的写数据），且有 write 回调
        // 后置: write 被调用进行刷新，然后转为读模式
        static WRITE_FLUSHED: AtomicBool = AtomicBool::new(false);

        unsafe extern "C" fn track_write(_f: *mut FILE, _buf: *const u8, _len: usize) -> usize {
            WRITE_FLUSHED.store(true, Ordering::SeqCst);
            0
        }

        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.wpos = buf.as_mut_ptr().add(8); // 写位置前移
            f.wbase = buf.as_mut_ptr();        // 基地址在起始
            f.write = Some(track_write);

            let result = __toread(&mut f as *mut FILE);
            assert_eq!(result, 0, "刷新后应成功转为读模式");
            assert!(WRITE_FLUSHED.load(Ordering::SeqCst), "write 回调应被调用");
            assert!(f.wpos.is_null(), "wpos 应被清空");
        }
    });

    test!("test_toread_pending_write_no_callback" {
        // 前置: wpos != wbase，但 write 回调为 None
        // 后置: 跳过刷新直接转为读模式（不影响结果）
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.wpos = buf.as_mut_ptr().add(4); // 模拟有写数据
            f.wbase = buf.as_mut_ptr();
            f.write = None; // 无 write 回调

            let result = __toread(&mut f as *mut FILE);
            assert_eq!(result, 0, "即使无 write 回调也应成功");
            assert!(f.wpos.is_null());
        }
    });

    test!("test_toread_mode_calculation" {
        // 验证 mode |= mode-1 行为
        let mut buf = [0u8; 64];
        unsafe {
            let mut f = make_test_file(buf.as_mut_ptr(), 64);
            f.mode = 0;
            let _ = __toread(&mut f as *mut FILE);
            assert_eq!(f.mode, -1, "mode 从 0 应变为 -1");
        }
    });
}
