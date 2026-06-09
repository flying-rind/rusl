use super::imports::wcsncat;
use test_framework::test;


test!("test_basic_concat" {
    unsafe {
        let mut buf = [0u32; 20];
        core::ptr::copy_nonoverlapping([97u32, 98, 0].as_ptr(), buf.as_mut_ptr(), 3); 
        let src = [99u32, 100, 101, 0];
        wcsncat(buf.as_mut_ptr(), src.as_ptr(), 2);
        assert_eq!(buf[0..5], [97, 98, 99, 100, 0]);
    }
});
