//! ungetc 字符推回集成测试

use core::ffi::c_int;
use super::imports::{fopen, fclose, fgetc, ungetc};
use test_framework::test;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}

// ---- ungetc 测试 ----

// musl ungetc 不检查 NULL FILE*, 跳过 NULL 测试

test!("ungetc_eof" {
    // 推回 EOF 应返回 EOF
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = ungetc(-1, f);
    assert_eq!(ret, -1, "ungetc(EOF) 应返回 EOF");
    fclose(f);
});

test!("ungetc_then_fgetc" {
    // 创建带内容的文件
    let path = b"/tmp/__rusl_test_ungetc__.dat\0";
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null());
    // 写入 'A'
    let _ = super::imports::fputc(b'A' as c_int, fw);
    fclose(fw);

    // 读取
    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null());

    // 读取 'A'
    let c1 = fgetc(fr);
    assert_eq!(c1, b'A' as c_int);

    // 推回 'B'
    let ret = ungetc(b'B' as c_int, fr);
    assert_eq!(ret, b'B' as c_int, "ungetc 应返回推回的字符");

    // 读取应得到 'B'
    let c2 = fgetc(fr);
    assert_eq!(c2, b'B' as c_int, "推回后应读回 'B'");

    fclose(fr);
});

test!("ungetc_getc_roundtrip" {
    let path = b"/tmp/__rusl_test_ungetc2__.dat\0";

    // 创建文件
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null());
    let _ = super::imports::fputc(b'X' as c_int, fw);
    fclose(fw);

    // 测试推回
    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null());

    let c1 = fgetc(fr);  // 'X'
    assert_eq!(c1, b'X' as c_int);

    let ret = ungetc(b'Z' as c_int, fr);
    assert_eq!(ret, b'Z' as c_int);

    let c2 = fgetc(fr);  // 'Z' (推回的)
    assert_eq!(c2, b'Z' as c_int);

    let _ = fgetc(fr);  // 应继续读取原内容或 EOF
    fclose(fr);
});
