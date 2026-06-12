//! fseeko / ftello / fgetpos / fsetpos 集成测试

use core::ffi::c_int;
use super::imports::{fopen, fclose, fseeko, ftello, fgetpos, fsetpos};
use test_framework::test;

// 常见 whence 值
const SEEK_SET: c_int = 0;
const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}

// ---- fseeko 测试 ----

// musl fseeko 不检查 NULL FILE*, 跳过 NULL 测试

test!("fseeko_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    // /dev/null 不可 seek, 但调用不应崩溃
    let ret = fseeko(f, 0, SEEK_SET);
    // 允许成功或失败, 取决于实现
    let _ = ret;
    fclose(f);
});

test!("fseeko_seek_set" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fseeko(f, 0, SEEK_SET);
    let _ = ret;
    fclose(f);
});

test!("fseeko_seek_cur" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fseeko(f, 0, SEEK_CUR);
    let _ = ret;
    fclose(f);
});

test!("fseeko_seek_end" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fseeko(f, 0, SEEK_END);
    let _ = ret;
    fclose(f);
});

// ---- ftello 测试 ----

// musl ftello 不检查 NULL FILE*, 跳过 NULL 测试

test!("ftello_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let pos = ftello(f);
    // /dev/null 不可 seek, 初始位置通常为 0 或不可用
    let _ = pos;
    fclose(f);
});

// ---- fgetpos / fsetpos 测试 ----

// musl fgetpos 调用 fseeko, fseeko 不检查 NULL, 跳过 NULL 测试

test!("fgetpos_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut pos: i64 = 0;
    let ret = fgetpos(f, &raw mut pos);
    // 允许成功或失败
    let _ = ret;
    fclose(f);
});

// musl fsetpos 调用 fseeko, fseeko 不检查 NULL, 跳过 NULL 测试

test!("fsetpos_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let pos: i64 = 0;
    let ret = fsetpos(f, &raw const pos);
    let _ = ret;
    fclose(f);
});

test!("fgetpos_fsetpos_roundtrip" {
    let path = b"/tmp/__rusl_test_fpos__.dat\0";
    // 创建文件并写入一些内容
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null());
    fclose(fw);

    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());

    // 获取初始位置
    let mut pos: i64 = 0;
    let ret = fgetpos(f, &raw mut pos);
    assert_eq!(ret, 0, "fgetpos 应返回 0");

    // 恢复到之前保存的位置
    let ret2 = fsetpos(f, &raw const pos);
    assert_eq!(ret2, 0, "fsetpos 应返回 0");

    fclose(f);
});
