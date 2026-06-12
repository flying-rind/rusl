//! GNU 扩展测试: fopencookie / fpurge 集成测试

use core::ffi::{c_char, c_void};
use super::imports::{fopencookie, fpurge, fclose, fopen};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- fopencookie 测试 ----

test!("fopencookie_basic" {
    use super::imports::cookie_io_functions_t;

    // 创建只有 write 回调的 cookie 流
    let io = cookie_io_functions_t {
        read: None,
        write: Some(dummy_write),
        seek: None,
        close: None,
    };

    let cookie: *mut c_void = core::ptr::null_mut();
    let f = fopencookie(cookie, cstr(b"w\0"), io);
    assert!(!f.is_null(), "fopencookie 应返回有效 FILE*");
    fclose(f);
});

// musl fopencookie 不检查 NULL mode, 跳过 NULL 测试

// fopencookie 写回调辅助函数
unsafe extern "C" fn dummy_write(
    _cookie: *mut c_void,
    _buf: *const c_char,
    size: usize,
) -> isize {
    size as isize
}

// ---- fpurge 测试 ----

test!("fpurge_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fpurge(f);
    assert_eq!(ret, 0, "fpurge 应返回 0");
    fclose(f);
});
