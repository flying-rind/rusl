use super::imports::wcsrchr;
use rusl_core::test;


test!("test_found" {
    unsafe {
        let s = [97u32, 98, 99, 98, 0];
        let r = wcsrchr(s.as_ptr(), 98);
        assert!(!r.is_null());
        assert_eq!(*r , 98);
    }
});
