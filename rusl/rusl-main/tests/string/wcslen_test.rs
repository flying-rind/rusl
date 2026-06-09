use super::imports::wcslen;
use test_framework::test;


test!("test_basic_length" {
    unsafe {
        let s = [97u32, 98, 99, 0];  // L"abc"
        let r = wcslen(s.as_ptr());
        assert_eq!(r, 3);
    }
});

test!("test_empty_string" {
    unsafe {
        let s = [0u32];
        let r = wcslen(s.as_ptr());
        assert_eq!(r, 0);
    }
});
