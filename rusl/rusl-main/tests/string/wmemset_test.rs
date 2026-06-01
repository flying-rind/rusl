use super::imports::wmemset;
use rusl_core::test;


test!("test_basic_set" {
    unsafe {
        let mut buf = [0u32; 10];
        wmemset(buf.as_mut_ptr(), 0xABCD, 10);
        assert_eq!(buf, [0xABCDu32; 10]);
    }
});
