use core::ffi::c_char;
use test_framework::test;

test!("test_ctermid_null" {
    {
        // ctermid(NULL) returns a pointer to the string "/dev/tty"
        let result = super::ctermid(core::ptr::null_mut());
        assert!(!result.is_null());
        unsafe {
            assert_eq!(*result, '/' as c_char);
        }
    }
});

test!("test_ctermid_buffer" {
    {
        let mut buf: [u8; 16] = [0u8; 16];
        let ptr = buf.as_mut_ptr() as *mut c_char;
        let result = super::ctermid(ptr);
        // Should return the same pointer we passed in
        assert_eq!(result, ptr);
        // Should contain "/dev/tty"
        assert_eq!(buf[0], b'/');
        assert_eq!(buf[1], b'd');
        assert_eq!(buf[2], b'e');
        assert_eq!(buf[3], b'v');
        assert_eq!(buf[4], b'/');
        assert_eq!(buf[5], b't');
        assert_eq!(buf[6], b't');
        assert_eq!(buf[7], b'y');
        assert_eq!(buf[8], 0u8);
    }
});
