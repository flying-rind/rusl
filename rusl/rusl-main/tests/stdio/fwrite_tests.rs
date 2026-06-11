//! fwrite 集成测试

use core::ffi::c_void;
use super::imports::fwrite;
use test_framework::test;

test!("fwrite_null_buffer_zero_size" {
    let n = fwrite(core::ptr::null(), 1, 0, core::ptr::null_mut());
    assert_eq!(n, 0);
});

test!("fwrite_zero_nmemb" {
    let data: [u8; 4] = [1, 2, 3, 4];
    let n = fwrite(data.as_ptr() as *const c_void, 4, 0, core::ptr::null_mut());
    assert_eq!(n, 0);
});
