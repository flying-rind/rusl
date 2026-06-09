use super::imports::wcsdup;
use test_framework::test;


test!("test_basic_dup" {
    unsafe {
        let s = [97u32, 98, 99, 0];
        let r = wcsdup(s.as_ptr());
        assert!(!r.is_null());
    }
});
