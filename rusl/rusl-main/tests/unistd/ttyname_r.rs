use core::ffi::c_char;
use test_framework::test;

test!("test_ttyname_r_invalid_fd" {
    {
        // -1 is not a valid fd, so ttyname_r should return an error code (non-zero)
        let mut buf: [u8; 256] = [0u8; 256];
        let ret = super::ttyname_r(-1, buf.as_mut_ptr() as *mut c_char, 256);
        assert_ne!(ret, 0);
    }
});
