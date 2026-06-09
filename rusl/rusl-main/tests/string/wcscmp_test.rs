use super::imports::wcscmp;
use test_framework::test;


test!("test_equal" {
    {
        let a = [97u32, 98, 99, 0]; let b = [97u32, 98, 99, 0];
        let r = wcscmp(a.as_ptr(), b.as_ptr());
        assert_eq!(r, 0);
    }
});

test!("test_different" {
    {
        let a = [97u32, 98, 99, 0]; let b = [97u32, 98, 100, 0];
        let r = wcscmp(a.as_ptr(), b.as_ptr());
        assert!(r < 0);
    }
});
