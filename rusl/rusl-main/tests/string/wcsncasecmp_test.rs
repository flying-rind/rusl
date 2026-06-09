use super::imports::wcsncasecmp;
use test_framework::test;


test!("test_equal" {
    {
        let a = [65u32, 66, 67, 68, 0]; let b = [97u32, 98, 99, 120, 0];
        let r = wcsncasecmp(a.as_ptr(), b.as_ptr(), 3);
        assert_eq!(r, 0);
    }
});
