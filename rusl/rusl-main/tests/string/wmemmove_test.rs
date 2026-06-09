use super::imports::wmemmove;
use test_framework::test;


test!("test_basic_copy" {
    {
        let src = [1u32, 2, 3, 4, 5]; let mut dst = [0u32; 5];
        let r = wmemmove(dst.as_mut_ptr(), src.as_ptr(), 5);
        assert_eq!(r, dst.as_mut_ptr());
        assert_eq!(dst, [1, 2, 3, 4, 5]);
    }
});
