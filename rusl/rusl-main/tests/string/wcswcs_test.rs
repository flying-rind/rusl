use super::imports::wcswcs;
use test_framework::test;


test!("test_found" {
    {
        let h = [97u32, 98, 99, 0]; let n = [98u32, 99, 0];
        let r = wcswcs(h.as_ptr(), n.as_ptr());
        assert!(!r.is_null());
    }
});
