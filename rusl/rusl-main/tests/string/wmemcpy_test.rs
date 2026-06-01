use super::imports::wmemcpy;
use rusl_core::test;


test!("test_basic_copy" {
    unsafe {
        let src = [1u32, 2, 3, 4, 5]; let mut dst = [0u32; 5];
        let r = wmemcpy(dst.as_mut_ptr(), src.as_ptr(), 5);
        assert_eq!(r, dst.as_mut_ptr());
        assert_eq!(dst, [1, 2, 3, 4, 5]);
    }
});

test!("test_zero_length" {
    unsafe {
        let src = [1u32; 5]; let mut dst = [0u32; 5];
        wmemcpy(dst.as_mut_ptr(), src.as_ptr(), 0);
        assert_eq!(dst, [0u32; 5]);
    }
});
