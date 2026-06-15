//! linkat 函数集成测试

use core::ffi::c_char;
use test_framework::test;

const AT_FDCWD: i32 = -100;

fn dirname_end(buf: &[u8], len: usize) -> usize {
    let mut pos = len;
    while pos > 0 {
        pos -= 1;
        if buf[pos] == b'/' {
            return pos + 1;
        }
    }
    0
}

test!("test_linkat_basic" {
    {
        // 通过 /proc/self/exe 获取可执行文件路径
        let mut src_buf: [u8; 256] = [0; 256];
        let src_path = b"/proc/self/exe\0";
        let len = super::readlink(
            src_path.as_ptr() as *const c_char,
            src_buf.as_mut_ptr() as *mut c_char,
            255,
        );
        assert!(len > 0);
        src_buf[len as usize] = 0;

        // 目标放在与二进制同目录中（同文件系统避免 EXDEV）
        let dir_end = dirname_end(&src_buf, len as usize);
        let suffix = b"rusl_test_linkat_dst\0";
        let mut dst_buf: [u8; 320] = [0; 320];
        for i in 0..dir_end {
            dst_buf[i] = src_buf[i];
        }
        for (i, &b) in suffix.iter().enumerate() {
            dst_buf[dir_end + i] = b;
        }

        let _ = super::unlink(dst_buf.as_ptr() as *const c_char);

        let ret = super::linkat(
            AT_FDCWD,
            src_buf.as_ptr() as *const c_char,
            AT_FDCWD,
            dst_buf.as_ptr() as *const c_char,
            0,
        );
        assert_eq!(ret, 0);

        let _ = super::unlink(dst_buf.as_ptr() as *const c_char);
    }
});

test!("test_linkat_noent" {
    {
        let src = b"/nonexistent_source_rusl_test\0";
        let dst = b"/nonexistent_dst_rusl_test\0";
        let ret = super::linkat(
            AT_FDCWD,
            src.as_ptr() as *const c_char,
            AT_FDCWD,
            dst.as_ptr() as *const c_char,
            0,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_linkat_invalid_fd" {
    {
        let src = b"nonexistent_src\0";
        let dst = b"nonexistent_dst\0";
        let ret = super::linkat(
            -1,
            src.as_ptr() as *const c_char,
            AT_FDCWD,
            dst.as_ptr() as *const c_char,
            0,
        );
        assert_eq!(ret, -1);
    }
});
