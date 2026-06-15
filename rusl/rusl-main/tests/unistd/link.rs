//! link 函数集成测试
//!
//! link 需要已存在的普通文件作为源。通过 readlink(/proc/self/exe)
//! 获取当前可执行文件路径作为源，并从其路径提取目录作为目标基础路径，
//! 确保源和目标在同一文件系统上，避免 EXDEV 错误。

use core::ffi::c_char;
use test_framework::test;

/// 在缓冲区中找到最后一个 '/' 的位置，返回其后一个字节的索引
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

test!("test_link_basic" {
    {
        // 读取 /proc/self/exe 获取可执行文件路径
        let mut src_buf: [u8; 256] = [0; 256];
        let src_path = b"/proc/self/exe\0";
        let len = super::readlink(
            src_path.as_ptr() as *const c_char,
            src_buf.as_mut_ptr() as *mut c_char,
            255,
        );
        assert!(len > 0);
        src_buf[len as usize] = 0;

        // 在二进制所在目录创建目标链接（同文件系统）
        let dir_end = dirname_end(&src_buf, len as usize);
        let suffix = b"rusl_test_link_dst\0";
        let mut dst_buf: [u8; 320] = [0; 320];
        for i in 0..dir_end {
            dst_buf[i] = src_buf[i];
        }
        for (i, &b) in suffix.iter().enumerate() {
            dst_buf[dir_end + i] = b;
        }

        let _ = super::unlink(dst_buf.as_ptr() as *const c_char);

        let ret = super::link(
            src_buf.as_ptr() as *const c_char,
            dst_buf.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        let _ = super::unlink(dst_buf.as_ptr() as *const c_char);
    }
});

test!("test_link_noent" {
    {
        let src = b"/nonexistent_source_rusl_test\0";
        let dst = b"/nonexistent_dst_rusl_test\0";
        let ret = super::link(
            src.as_ptr() as *const c_char,
            dst.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_link_eexist" {
    {
        // 尝试链接到已存在路径 (源=目标)
        let src = b"/dev/null\0";
        let ret = super::link(
            src.as_ptr() as *const c_char,
            src.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_link_directory" {
    {
        // link 不能对目录创建硬链接 (除了 root)
        let src = b"/\0";
        let dst = b"/nonexistent_dst_dir_test\0";
        let ret = super::link(
            src.as_ptr() as *const c_char,
            dst.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);
    }
});
