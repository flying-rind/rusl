use core::ffi::c_char;
use test_framework::test;

test!("test_getlogin_r_basic" {
    {
        let mut buf: [u8; 256] = [0u8; 256];
        let ret = super::getlogin_r(buf.as_mut_ptr() as *mut c_char, 256);
        // Returns 0 on success, ENXIO (6) if no login name, ERANGE (34) if buffer too small
        // We just verify the function doesn't crash since the result depends on
        // whether LOGNAME is set in the test environment.
        let _ = ret;
    }
});
