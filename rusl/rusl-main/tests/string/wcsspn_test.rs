use super::imports::wcsspn;
use test_framework::test;


test!("test_basic" {
    unsafe {
        let s = [97u32, 98, 99, 100, 0]; let accept = [97u32, 98, 99, 0];
        let r = wcsspn(s.as_ptr(), accept.as_ptr());
        assert_eq!(r, 3);
    }
});
