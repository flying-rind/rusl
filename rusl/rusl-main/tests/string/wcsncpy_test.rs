use super::imports::wcsncpy;
use test_framework::test;


test!("test_basic_copy" {
    unsafe {
        let src = [97u32, 98, 99, 0]; let mut dst = [0u32; 10];
        wcsncpy(dst.as_mut_ptr(), src.as_ptr(), 10);
        assert_eq!(dst[0..4], [97, 98, 99, 0]);
    }
});
