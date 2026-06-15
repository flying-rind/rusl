//! readlinkat 函数集成测试

use core::ffi::c_char;
use test_framework::test;

const AT_FDCWD: i32 = -100;

test!("test_readlinkat_basic" {
    {
        let target = b"readlinkat_target\0";
        let name = b"/tmp/rusl_test_readlinkat\0";
        let _ = super::symlinkat(
            target.as_ptr() as *const c_char,
            AT_FDCWD,
            name.as_ptr() as *const c_char,
        );

        let mut buf: [u8; 256] = [0; 256];
        let len = super::readlinkat(
            AT_FDCWD,
            name.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert!(len > 0);
        assert_eq!(len as usize, b"readlinkat_target".len());

        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_readlinkat_bufsize_zero" {
    {
        let target = b"readlinkat_zero\0";
        let name = b"/tmp/rusl_test_readlinkat_z\0";
        let _ = super::symlinkat(
            target.as_ptr() as *const c_char,
            AT_FDCWD,
            name.as_ptr() as *const c_char,
        );

        let mut buf: [u8; 1] = [0; 1];
        let len = super::readlinkat(
            AT_FDCWD,
            name.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            0,
        );
        assert_eq!(len, 0);

        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_readlinkat_noent" {
    {
        let path = b"/nonexistent_link_rusl_test\0";
        let mut buf: [u8; 256] = [0; 256];
        let len = super::readlinkat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert_eq!(len, -1);
    }
});

test!("test_readlinkat_invalid_fd" {
    {
        let path = b"nonexistent_link_rusl_test\0";
        let mut buf: [u8; 256] = [0; 256];
        let len = super::readlinkat(
            -1,
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert_eq!(len, -1);
    }
});
