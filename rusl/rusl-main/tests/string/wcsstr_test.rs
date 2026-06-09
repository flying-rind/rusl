use super::imports::wcsstr;
use test_framework::test;


test!("test_found" {
    {
        let h = [97u32, 98, 99, 100, 0]; let n = [99u32, 100, 0];
        let r = wcsstr(h.as_ptr(), n.as_ptr());
        assert!(!r.is_null());
    }
});

test!("test_not_found" {
    {
        let h = [97u32, 98, 99, 0]; let n = [120u32, 121, 0];
        let r = wcsstr(h.as_ptr(), n.as_ptr());
        assert!(r.is_null());
    }
});
