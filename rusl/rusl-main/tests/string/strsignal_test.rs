use super::imports::strsignal;
use test_framework::test;


test!("test_valid_signal" {
    {
        let r = strsignal(1);
        assert!(!r.is_null());
    }
});
