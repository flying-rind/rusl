use super::imports::wmemset;
use test_framework::test;


test!("test_basic_set" {
    {
        let mut buf = [0u32; 10];
        wmemset(buf.as_mut_ptr(), 0xABCD, 10);
        assert_eq!(buf, [0xABCDu32; 10]);
    }
});
