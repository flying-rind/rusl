use super::imports::wcscasecmp;
use test_framework::test;


test!("test_equal" {
    unsafe {
        let a = [65u32, 66, 67, 0]; let b = [97u32, 98, 99, 0];
        let r = wcscasecmp(a.as_ptr(), b.as_ptr());
        assert_eq!(r, 0);
    }
});
