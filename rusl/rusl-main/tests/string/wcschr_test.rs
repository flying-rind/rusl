use super::imports::wcschr;
use test_framework::test;


test!("test_found" {
    unsafe {
        let s = [97u32, 98, 99, 0];
        let r = wcschr(s.as_ptr(), 98);
        assert!(!r.is_null());
        assert_eq!( *r , 98);
    }
});

test!("test_not_found" {
    {
        let s = [97u32, 98, 99, 0];
        let r = wcschr(s.as_ptr(), 120);
        assert!(r.is_null());
    }
});
