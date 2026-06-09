use super::imports::wcsncasecmp_l;
use test_framework::test;


test!("test_equal" {
    unsafe {
        let a = [65u32, 66, 67, 68, 0]; let b = [97u32, 98, 99, 120, 0];
        let r = wcsncasecmp_l(a.as_ptr(), b.as_ptr(), 3, core::ptr::null_mut());
        assert_eq!(r, 0);
    }
});
