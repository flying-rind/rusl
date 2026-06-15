//! readlink 函数集成测试

use core::ffi::c_char;
use test_framework::test;

test!("test_readlink_basic" {
    {
        // 创建符号链接
        let target = b"hello_target\0";
        let name = b"/tmp/rusl_test_readlink\0";
        let ret = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        // 读取符号链接
        let mut buf: [u8; 256] = [0; 256];
        let len = super::readlink(
            name.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert!(len > 0);
        // 返回的长度应等于目标字符串长度（不含 '\0'）
        assert_eq!(len as usize, b"hello_target".len());

        // 清理
        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_readlink_content" {
    {
        let target = b"some/path/to/target\0";
        let name = b"/tmp/rusl_test_readlink2\0";
        let _ = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );

        let mut buf: [u8; 256] = [0; 256];
        let len = super::readlink(
            name.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert!(len > 0);
        assert_eq!(len as usize, b"some/path/to/target".len());
        // 验证内容
        for i in 0..len as usize {
            assert_eq!(buf[i], target[i]);
        }

        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_readlink_bufsize_zero" {
    {
        // bufsize=0: 验证符号链接存在，但不读入缓冲区
        let target = b"bufsize_zero_target\0";
        let name = b"/tmp/rusl_test_readlink_q\0";
        let _ = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );

        let mut buf: [u8; 1] = [0; 1];
        let len = super::readlink(
            name.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            0,
        );
        // musl 特殊处理：bufsize==0 时返回 0
        assert_eq!(len, 0);

        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_readlink_small_buffer" {
    {
        // 目标比缓冲区大，应截断
        let target = b"this_is_a_long_target_string\0";
        let name = b"/tmp/rusl_test_readlink_trunc\0";
        let _ = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );

        let mut buf: [u8; 10] = [0; 10];
        let len = super::readlink(
            name.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            10,
        );
        // 缓冲区大小 10，最多读入 10 字节
        assert!(len >= 0);
        assert!(len <= 10);

        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_readlink_not_symlink" {
    {
        // /dev/null 不是符号链接，readlink 应失败
        let path = b"/dev/null\0";
        let mut buf: [u8; 256] = [0; 256];
        let len = super::readlink(
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert_eq!(len, -1);
    }
});

test!("test_readlink_noent" {
    {
        let path = b"/nonexistent_link_rusl_test\0";
        let mut buf: [u8; 256] = [0; 256];
        let len = super::readlink(
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert_eq!(len, -1);
    }
});
