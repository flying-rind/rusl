use super::imports::wcspbrk;
use rusl_core::test;


test!("test_found" {
    unsafe {
        let s = [97u32, 98, 99, 0]; let accept = [99u32, 100, 0];
        let r = wcspbrk(s.as_ptr(), accept.as_ptr());
        assert!(!r.is_null());
        assert_eq!( *r , 99);
    }
});

test!("test_not_found" {
    unsafe {
        let s = [97u32, 98, 99, 0]; let accept = [120u32, 121, 0];
        let r = wcspbrk(s.as_ptr(), accept.as_ptr());
        assert!(r.is_null());
    }
});
