use core::ffi::{c_void};
use super::imports::memmem;
use test_framework::test;


test!("test_found" {
    unsafe {
        let haystack = [1u8, 2, 3, 4, 5]; let needle = [3u8, 4];
        let r = memmem(haystack.as_ptr() as *const c_void, 5, needle.as_ptr() as *const c_void, 2);
        assert!(!r.is_null());
        assert_eq!(*(r as *const u8) , 3);
    }
});

test!("test_not_found" {
    {
        let haystack = [1u8, 2, 3]; let needle = [4u8, 5];
        let r = memmem(haystack.as_ptr() as *const c_void, 3, needle.as_ptr() as *const c_void, 2);
        assert!(r.is_null());
    }
});

test!("test_empty_needle" {
    {
        let haystack = [1u8, 2, 3];
        let r = memmem(haystack.as_ptr() as *const c_void, 3, core::ptr::null::<c_void>(), 0);
        assert_eq!(r, haystack.as_ptr() as *mut c_void);
    }
});
