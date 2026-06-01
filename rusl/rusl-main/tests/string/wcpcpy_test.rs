use super::imports::wcpcpy;
use rusl_core::test;


test!("test_basic_copy" {
    unsafe {
        let s = [97u32, 98, 99, 0]; let mut d = [0u32; 10];
        let r = wcpcpy(d.as_mut_ptr(), s.as_ptr());
        let expected = d.as_mut_ptr().add(3);
        assert_eq!(r, expected);
    }
});
