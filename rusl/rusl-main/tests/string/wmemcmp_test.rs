use super::imports::wmemcmp;
use rusl_core::test;


test!("test_equal" {
    unsafe {
        let a = [1u32, 2, 3]; let b = [1u32, 2, 3];
        let r = wmemcmp(a.as_ptr(), b.as_ptr(), 3);
        assert_eq!(r, 0);
    }
});

test!("test_different" {
    unsafe {
        let a = [1u32, 2, 3]; let b = [1u32, 2, 4];
        let r = wmemcmp(a.as_ptr(), b.as_ptr(), 3);
        assert!(r < 0);
    }
});
