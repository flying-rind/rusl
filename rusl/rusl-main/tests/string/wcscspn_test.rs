use super::imports::wcscspn;
use test_framework::test;


test!("test_basic" {
    {
        let s = [97u32, 98, 99, 100, 101, 0]; let reject = [100u32, 101, 0];
        let r = wcscspn(s.as_ptr(), reject.as_ptr());
        assert_eq!(r, 3);
    }
});
