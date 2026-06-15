use core::ffi::c_char;
use test_framework::test;

test!("test_gethostname_basic" {
    {
        let mut buf: [u8; 256] = [0u8; 256];
        let ret = super::gethostname(buf.as_mut_ptr() as *mut c_char, 256);
        assert_eq!(ret, 0);
        // The buffer should be null-terminated with a non-empty hostname
        let len = buf.iter().position(|&x| x == 0).unwrap_or(256);
        assert!(len > 0);      // hostname should not be empty
        assert!(len < 256);    // null terminator should be within buffer
    }
});

test!("test_gethostname_truncation" {
    {
        let mut buf: [u8; 1] = [0u8; 1];
        let ret = super::gethostname(buf.as_mut_ptr() as *mut c_char, 1);
        assert_eq!(ret, 0);
        // When hostname is longer than the buffer, it's truncated and
        // the last byte is set to '\0'
        assert_eq!(buf[0], 0u8);
    }
});
