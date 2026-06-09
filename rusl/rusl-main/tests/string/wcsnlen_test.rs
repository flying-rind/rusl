use super::imports::wcsnlen;
use test_framework::test;


test!("test_basic" {
    {
        let s = [97u32, 98, 99, 0];
        let r = wcsnlen(s.as_ptr(), 10);
        assert_eq!(r, 3);
    }
});

test!("test_limited" {
    {
        let s = [97u32, 98, 99, 100, 0];
        let r = wcsnlen(s.as_ptr(), 2);
        assert_eq!(r, 2);
    }
});
