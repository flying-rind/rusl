use super::imports::wcstok;
use test_framework::test;


test!("test_first_token" {
    unsafe {
        let mut buf = [97u32, 98, 0, 99, 100, 0];
        let sep = [32u32, 0];
        let mut state: *mut u32 = core::ptr::null_mut();
        let r = wcstok(buf.as_mut_ptr(), sep.as_ptr(), &mut state);
        assert!(!r.is_null());
    }
});
