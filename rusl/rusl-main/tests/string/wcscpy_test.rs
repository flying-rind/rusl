use super::imports::wcscpy;
use rusl_core::test;


test!("test_basic_copy" {
    unsafe {
        let s = [97u32, 98, 99, 0]; let mut d = [0u32; 10];
        let r = wcscpy(d.as_mut_ptr(), s.as_ptr());
        assert_eq!(r, d.as_mut_ptr());
        assert_eq!(d[0..4], [97, 98, 99, 0]);
    }
});
