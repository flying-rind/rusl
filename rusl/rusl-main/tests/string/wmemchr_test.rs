use super::imports::wmemchr;
use test_framework::test;


test!("test_found" {
    unsafe {
        let buf = [10u32, 20, 30, 40];
        let r = wmemchr(buf.as_ptr(), 30, 4);
        assert!(!r.is_null());
        assert_eq!(*r , 30);
    }
});

test!("test_not_found" {
    {
        let buf = [10u32, 20, 30];
        let r = wmemchr(buf.as_ptr(), 99, 3);
        assert!(r.is_null());
    }
});
