use super::imports::wcsncmp;
use rusl_core::test;


test!("test_equal" {
    unsafe {
        let a = [97u32, 98, 99, 0]; let b = [97u32, 98, 99, 0];
        let r = wcsncmp(a.as_ptr(), b.as_ptr(), 3);
        assert_eq!(r, 0);
    }
});
