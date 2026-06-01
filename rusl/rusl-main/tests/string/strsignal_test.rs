use super::imports::strsignal;
use rusl_core::test;


test!("test_valid_signal" {
    unsafe {
        let r = strsignal(1);
        assert!(!r.is_null());
    }
});
