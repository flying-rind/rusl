//! getcwd 函数集成测试

use core::ffi::c_char;
use test_framework::test;

test!("test_getcwd_basic" {
    {
        let mut buf: [u8; 4096] = [0; 4096];
        let result = super::getcwd(
            buf.as_mut_ptr() as *mut c_char,
            4096,
        );
        // getcwd should return the buffer address on success
        assert!(!result.is_null());
        // The result should point to our buffer
        assert_eq!(result, buf.as_mut_ptr() as *mut c_char);
        // The path should start with '/'
        assert_eq!(buf[0], b'/');
    }
});

test!("test_getcwd_small_buffer" {
    {
        // With a reasonably sized buffer
        let mut buf: [u8; 256] = [0; 256];
        let result = super::getcwd(
            buf.as_mut_ptr() as *mut c_char,
            256,
        );
        assert!(!result.is_null());
        assert_eq!(buf[0], b'/');
    }
});

test!("test_getcwd_size_zero" {
    {
        // size=0 with non-null buf should return NULL with EINVAL
        let mut buf: [u8; 1] = [0; 1];
        let result = super::getcwd(
            buf.as_mut_ptr() as *mut c_char,
            0,
        );
        assert!(result.is_null());
    }
});

test!("test_getcwd_null_buf" {
    {
        // buf=NULL: getcwd should allocate memory and return the path
        let result = super::getcwd(
            core::ptr::null_mut(),
            0,
        );
        assert!(!result.is_null());
    }
});
