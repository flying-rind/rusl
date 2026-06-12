//! scanf / fscanf / sscanf 集成测试
//!
//! sscanf 最容易测试（字符串输入）, scanf/fscanf 做基本烟雾测试。

use core::ffi::{c_char, c_int};
use super::imports::{sscanf, scanf, fscanf, fopen, fclose};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- sscanf 测试 ----

test!("sscanf_int" {
    let mut val: c_int = 0;
    let ret = unsafe {
        sscanf(
            cstr(b"42\0"),
            cstr(b"%d\0"),
            &mut val as *mut c_int,
        )
    };
    assert_eq!(ret, 1, "应成功解析 1 个整数");
    assert_eq!(val, 42);
});

test!("sscanf_two_ints" {
    let mut a: c_int = 0;
    let mut b: c_int = 0;
    let ret = unsafe {
        sscanf(
            cstr(b"10 20\0"),
            cstr(b"%d %d\0"),
            &mut a as *mut c_int,
            &mut b as *mut c_int,
        )
    };
    assert_eq!(ret, 2, "应成功解析 2 个整数");
    assert_eq!(a, 10);
    assert_eq!(b, 20);
});

test!("sscanf_negative" {
    let mut val: c_int = 0;
    let ret = unsafe {
        sscanf(
            cstr(b"-123\0"),
            cstr(b"%d\0"),
            &mut val as *mut c_int,
        )
    };
    assert_eq!(ret, 1);
    assert_eq!(val, -123);
});

test!("sscanf_no_match" {
    let mut val: c_int = 0;
    let ret = unsafe {
        sscanf(
            cstr(b"abc\0"),
            cstr(b"%d\0"),
            &mut val as *mut c_int,
        )
    };
    assert_eq!(ret, 0, "无法匹配时返回 0");
    // val 未改变
    assert_eq!(val, 0);
});

test!("sscanf_str" {
    let mut buf: [u8; 16] = [0; 16];
    let ret = unsafe {
        sscanf(
            cstr(b"hello\0"),
            cstr(b"%s\0"),
            buf.as_mut_ptr() as *mut c_char,
        )
    };
    assert_eq!(ret, 1, "应成功解析 1 个字符串");
    assert_eq!(&buf[..5], b"hello");
});

test!("sscanf_hex" {
    let mut val: c_int = 0;
    let ret = unsafe {
        sscanf(
            cstr(b"ff\0"),
            cstr(b"%x\0"),
            &mut val as *mut c_int,
        )
    };
    assert_eq!(ret, 1);
    assert_eq!(val, 255);
});

// ---- fscanf 测试 ----

test!("fscanf_from_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut val: c_int = 0;
    let ret = unsafe {
        fscanf(f, cstr(b"%d\0"), &mut val as *mut c_int)
    };
    assert_eq!(ret, -1, "/dev/null 中无数据, 应返回 EOF");
    fclose(f);
});

// musl fscanf 不检查 NULL FILE*, 跳过 NULL FILE* 测试

// ---- scanf 烟雾测试 ----

// musl scanf 不检查 NULL fmt, 跳过 NULL fmt 测试
