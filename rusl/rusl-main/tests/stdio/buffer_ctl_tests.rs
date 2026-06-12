//! setbuffer / setlinebuf 集成测试

use core::ffi::c_char;
use super::imports::{fopen, fclose, setbuffer, setlinebuf};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- setbuffer 测试 ----

test!("setbuffer_null_buffer" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    // NULL 缓冲区 + size 0 = 无缓冲
    setbuffer(f, core::ptr::null_mut(), 0);
    fclose(f);
});

test!("setbuffer_with_size" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 256] = [0; 256];
    setbuffer(f, buf.as_mut_ptr() as *mut c_char, 256);
    fclose(f);
});

// musl 的 setbuffer/setlinebuf 对 NULL FILE* 会解引用导致 SIGSEGV

test!("setbuffer_small_size" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 1] = [0; 1];
    setbuffer(f, buf.as_mut_ptr() as *mut c_char, 1);
    fclose(f);
});

// ---- setlinebuf 测试 ----

test!("setlinebuf_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    setlinebuf(f);
    fclose(f);
});

// musl 的 setlinebuf 对 NULL FILE* 会解引用导致 SIGSEGV
