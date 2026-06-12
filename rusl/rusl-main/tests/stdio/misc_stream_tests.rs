//! fwide / perror 集成测试

use core::ffi::c_char;
use super::imports::{fopen, fclose, fwide, perror};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- fwide 测试 ----

test!("fwide_query_mode" {
    // mode=0: 查询当前方向
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fwide(f, 0);
    // 新流通常无方向 (返回 0)
    let _ = ret;
    fclose(f);
});

test!("fwide_set_byte" {
    // mode < 0: 设置为字节方向
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fwide(f, -1);
    // 返回负数表示设置为字节方向
    assert!(ret <= 0, "fwide(f, -1) 应返回 <= 0");
    fclose(f);
});

test!("fwide_set_wide" {
    // mode > 0: 尝试设置为宽字符方向
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fwide(f, 1);
    // 返回正数表示设置为宽字符方向; 如果流已有字节方向则返回负数
    let _ = ret;
    fclose(f);
});

// ---- perror 测试 ----

test!("perror_null_msg" {
    // NULL 消息: 不应崩溃
    perror(core::ptr::null());
});

test!("perror_non_null_msg" {
    // 非 NULL 消息: 不应崩溃
    perror(cstr(b"test\0"));
});

test!("perror_empty_msg" {
    // 空字符串
    perror(cstr(b"\0"));
});
