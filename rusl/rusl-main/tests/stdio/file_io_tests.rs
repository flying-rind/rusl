//! fread / fgets / fputs / fflush 集成测试

use core::ffi::{c_char, c_void};
use super::imports::{fopen, fclose, fread, fgets, fputs, fflush, fwrite};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- fread 测试 ----

test!("fread_null_buffer_zero_size" {
    // 前置: fread 用于空指针但 size 为 0
    // 后置: 返回 0
    let n = fread(core::ptr::null_mut(), 1, 0, core::ptr::null_mut());
    assert_eq!(n, 0);
});

test!("fread_zero_nmemb" {
    // 前置: nmemb = 0（零个元素）
    // 后置: 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 8] = [0; 8];
    let n = fread(buf.as_mut_ptr() as *mut c_void, 4, 0, f);
    assert_eq!(n, 0);
    fclose(f);
});

test!("fread_zero_size" {
    // 前置: size = 0（零字节大小）
    // 后置: 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 8] = [0; 8];
    let n = fread(buf.as_mut_ptr() as *mut c_void, 0, 10, f);
    assert_eq!(n, 0);
    fclose(f);
});

test!("fread_from_dev_null" {
    // 前置: 从 /dev/null 读取总是返回 EOF
    // 后置: 返回 0（无数据可读）
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 16] = [0xA5; 16];
    let n = fread(buf.as_mut_ptr() as *mut c_void, 1, 16, f);
    assert_eq!(n, 0, "/dev/null 应无数据可读");
    fclose(f);
});

test!("fread_null_file" {
    // 前置: FILE* 为 NULL
    // 后置: 返回 0
    let mut buf: [u8; 8] = [0; 8];
    let n = fread(buf.as_mut_ptr() as *mut c_void, 1, 8, core::ptr::null_mut());
    assert_eq!(n, 0);
});

// ---- fgets 测试 ----

test!("fgets_null_buffer" {
    // 前置: s 为 NULL 但 n <= 0
    // 后置: 返回 NULL
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fgets(core::ptr::null_mut(), 0, f);
    assert!(ret.is_null());
    fclose(f);
});

test!("fgets_zero_n" {
    // 前置: n = 0
    // 后置: 返回 NULL
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 8] = [0; 8];
    let ret = fgets(buf.as_mut_ptr() as *mut c_char, 0, f);
    assert!(ret.is_null());
    fclose(f);
});

test!("fgets_negative_n" {
    // 前置: n < 0
    // 后置: 返回 NULL
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 8] = [0; 8];
    let ret = fgets(buf.as_mut_ptr() as *mut c_char, -1, f);
    assert!(ret.is_null());
    fclose(f);
});

test!("fgets_null_file" {
    // 前置: FILE* 为 NULL
    // 后置: 返回 NULL
    let mut buf: [u8; 16] = [0; 16];
    let ret = fgets(buf.as_mut_ptr() as *mut c_char, 16, core::ptr::null_mut());
    assert!(ret.is_null());
});

// ---- fputs 测试 ----

test!("fputs_null_string" {
    // 前置: s 为 NULL
    // 后置: 返回 EOF
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fputs(core::ptr::null(), f);
    assert_eq!(ret, -1, "NULL 字符串应返回 EOF");
    fclose(f);
});

test!("fputs_null_file" {
    // 前置: FILE* 为 NULL
    // 后置: 返回 EOF
    let ret = fputs(cstr(b"test\0"), core::ptr::null_mut());
    assert_eq!(ret, -1, "NULL FILE* 应返回 EOF");
});

test!("fputs_empty_string" {
    // 前置: 空字符串（仅 '\0'）
    // 后置: 返回 0 或非负数
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fputs(cstr(b"\0"), f);
    assert!(ret >= 0, "空字符串写入应成功, got {}", ret);
    fclose(f);
});

// ---- fwrite + fread 回环测试 ----

test!("fwrite_fread_roundtrip" {
    // 前置: 创建临时文件
    // 后置: fwrite 写入的数据可被 fread 读取
    let path = b"/tmp/__rusl_test_roundtrip__.dat\0";

    // 写入数据
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null(), "无法创建临时文件");
    let data = b"Hello Rust stdio!\n";
    let _written = fwrite(data.as_ptr() as *const c_void, 1, data.len(), fw);
    fclose(fw);

    // 读取数据
    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null(), "无法打开临时文件");
    let mut buf: [u8; 32] = [0; 32];
    let read = fread(buf.as_mut_ptr() as *mut c_void, 1, 32, fr);
    assert!(read >= data.len(), "应读取至少 {} 字节, got {}", data.len(), read);
    assert_eq!(&buf[..data.len()], data, "写入和读取内容应一致");
    fclose(fr);
});

// ---- fflush 测试 ----

test!("fflush_null_file_flush_all" {
    // 前置: f 为 NULL（刷新所有流）
    // 后置: 返回 0
    let ret = fflush(core::ptr::null_mut());
    assert_eq!(ret, 0, "fflush(NULL) 应返回 0");
});

test!("fflush_valid_file" {
    // 前置: 有效 FILE*
    // 后置: 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fflush(f);
    assert_eq!(ret, 0, "fflush(有效 FILE*) 应返回 0");
    fclose(f);
});
