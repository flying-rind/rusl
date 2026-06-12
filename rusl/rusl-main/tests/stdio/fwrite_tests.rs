//! fwrite 集成测试

use core::ffi::c_void;
use super::imports::{fopen, fclose, fwrite};
use test_framework::test;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}

test!("fwrite_null_buffer_zero_size" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let n = fwrite(core::ptr::null(), 1, 0, f);
    assert_eq!(n, 0);
    fclose(f);
});

test!("fwrite_zero_nmemb" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let data: [u8; 4] = [1, 2, 3, 4];
    let n = fwrite(data.as_ptr() as *const c_void, 4, 0, f);
    assert_eq!(n, 0);
    fclose(f);
});
