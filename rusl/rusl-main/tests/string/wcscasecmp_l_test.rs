use super::imports::wcscasecmp_l;
use rusl_core::test;


test!("test_equal" {
    unsafe {
        let a = [65u32, 66, 67, 0]; let b = [97u32, 98, 99, 0];
        let r = wcscasecmp_l(a.as_ptr(), b.as_ptr(), core::ptr::null_mut());
        assert_eq!(r, 0);
    }
});
