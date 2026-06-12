//! getw / putw 集成测试
//!
//! getw/putw 以二进制方式读写 int (平台相关尺寸)。

use core::ffi::c_int;
use super::imports::{fopen, fclose, getw, putw};
use test_framework::test;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}

// ---- getw 测试 ----

// musl getw 调用 fread, fread 不检查 NULL, 跳过 NULL 测试

test!("getw_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = getw(f);
    // /dev/null 无数据, 返回 EOF
    assert_eq!(ret, -1, "/dev/null 的 getw 应返回 EOF");
    fclose(f);
});

// ---- putw 测试 ----

// musl putw 调用 fwrite, fwrite 不检查 NULL, 跳过 NULL 测试

test!("putw_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = putw(0x12345678u32 as c_int, f);
    // /dev/null 可写, 返回 0 (成功)
    assert_eq!(ret, 0, "putw 写 /dev/null 应返回 0");
    fclose(f);
});

test!("putw_getw_roundtrip" {
    let path = b"/tmp/__rusl_test_getw_putw__.dat\0";

    // 写入 int
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null());
    let val: c_int = 0x42;
    let ret_w = putw(val, fw);
    assert_eq!(ret_w, 0, "putw 应返回 0");
    fclose(fw);

    // 读取 int
    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null());
    let ret_r = getw(fr);
    assert_eq!(ret_r, val, "getw 应读回写入的值 {} vs {}", ret_r, val);
    fclose(fr);
});
