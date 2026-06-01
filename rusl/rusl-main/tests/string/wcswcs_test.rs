use super::imports::wcswcs;
use rusl_core::test;


test!("test_found" {
    unsafe {
        let h = [97u32, 98, 99, 0]; let n = [98u32, 99, 0];
        let r = wcswcs(h.as_ptr(), n.as_ptr());
        assert!(!r.is_null());
    }
});
