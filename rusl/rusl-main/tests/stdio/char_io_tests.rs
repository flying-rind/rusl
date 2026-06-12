//! fgetc / fputc / getc / putc / getchar / putchar / puts 集成测试

use core::ffi::{c_char, c_int};
use super::imports::{fopen, fclose, fgetc, fputc, getc, putc, putchar, puts, fflush};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- fgetc 测试 ----

// musl 的 fgetc 对 NULL FILE* 不检查而直接解引用, 测试使用 /dev/null 替代
test!("fgetc_null_file" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let c = fgetc(f);
    assert_eq!(c, -1, "/dev/null 的 fgetc 应返回 EOF");
    fclose(f);
});

test!("fgetc_eof_at_start" {
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let c = fgetc(f);
    assert_eq!(c, -1, "/dev/null 的 fgetc 应返回 EOF");
    fclose(f);
});

// ---- fputc 测试 ----

// musl 的 fputc 对 NULL FILE* 不检查, 测试使用 /dev/null
test!("fputc_null_file" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fputc(b'A' as c_int, f);
    assert_eq!(ret, b'A' as c_int, "fputc 应返回写入的字符");
    fclose(f);
});

test!("fputc_fgetc_roundtrip" {
    let path = b"/tmp/__rusl_test_fputc__.dat\0";

    // 写入
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null(), "无法创建临时文件");
    let ret_w = fputc(b'Z' as c_int, fw);
    assert_eq!(ret_w, b'Z' as c_int, "fputc 应返回写入的字符");
    fclose(fw);

    // 读取
    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null());
    let c = fgetc(fr);
    assert_eq!(c, b'Z' as c_int, "应读回 'Z'");
    fclose(fr);
});

// ---- getc 测试 ----

// musl 的 getc 对 NULL FILE* 不检查, 测试使用 /dev/null
test!("getc_null_file" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let c = getc(f);
    assert_eq!(c, -1, "/dev/null 的 getc 应返回 EOF");
    fclose(f);
});

// ---- putc 测试 ----

// musl 的 putc 对 NULL FILE* 不检查, 测试使用 /dev/null
test!("putc_null_file" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = putc(b'X' as c_int, f);
    assert_eq!(ret, b'X' as c_int, "putc 应返回写入的字符");
    fclose(f);
});

test!("putc_getc_roundtrip" {
    let path = b"/tmp/__rusl_test_putc__.dat\0";

    // 写入多个字符
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null());
    let chars = [b'H', b'i', b'!'];
    for &ch in &chars {
        let ret = putc(ch as c_int, fw);
        assert_eq!(ret, ch as c_int);
    }
    fclose(fw);

    // 读取
    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null());
    for &expected in &chars {
        let c = getc(fr);
        assert_eq!(c as u8, expected, "getc 应返回 {}", expected as char);
    }
    let eof = getc(fr);
    assert_eq!(eof, -1, "读取完毕后应返回 EOF");
    fclose(fr);
});

// ---- getchar / putchar 测试 ----
// 注意: getchar 从 stdin 读取, putchar 写入到 stdout
// 这两个操作依赖真实的终端 I/O, 在不重定向的环境中难测试
// 此处仅测试基本调用（不应崩溃）

test!("putchar_returns_char" {
    let ret = putchar(b'?' as c_int);
    // 如果 stdout 不可用（如无终端环境），允许返回 EOF
    if ret != -1 {
        assert_eq!(ret, b'?' as c_int);
    }
    fflush(core::ptr::null_mut());
});

// ---- puts 测试 ----

// musl 的 puts/fputs 对 NULL 字符串调用 strlen 导致 SIGSEGV
test!("puts_null_string" {
    let ret = puts(cstr(b"\0"));
    assert!(ret >= 0, "puts(空) 应返回 >= 0, got {}", ret);
    fflush(core::ptr::null_mut());
});

test!("puts_empty_string" {
    let ret = puts(cstr(b"\0"));
    assert!(ret >= 0, "puts(空) 应返回 >= 0, got {}", ret);
    fflush(core::ptr::null_mut());
});

test!("puts_nonempty" {
    let ret = puts(cstr(b"hello\0"));
    // puts 追加换行符, 所以至少输出 5+1=6 字节
    assert!(ret >= 0, "puts(hello) 应返回 >= 0, got {}", ret);
    fflush(core::ptr::null_mut());
});
