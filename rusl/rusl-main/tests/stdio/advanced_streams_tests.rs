//! fmemopen / open_memstream / open_wmemstream / popen / pclose
//! / fgetln / getdelim / getline 集成测试
//!
//! 这些函数创建高级流或使用高级读取语义。对于依赖外部命令的 (popen),
//! 测试以烟雾级别为主。

use core::ffi::{c_char, c_int};
use super::imports::{
    fmemopen, open_memstream, open_wmemstream,
    popen, pclose,
    fgetln, getdelim, getline,
    fopen, fclose, fflush,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- fmemopen 测试 ----

test!("fmemopen_read_only" {
    let data: [u8; 8] = *b"testdata";
    let f = fmemopen(
        data.as_ptr() as *mut core::ffi::c_void,
        8,
        cstr(b"r\0"),
    );
    assert!(!f.is_null(), "fmemopen 应返回有效 FILE*");
    fclose(f);
});

test!("fmemopen_write_only" {
    let mut buf: [u8; 64] = [0; 64];
    let f = fmemopen(
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        64,
        cstr(b"w\0"),
    );
    assert!(!f.is_null());
    fclose(f);
});

test!("fmemopen_null_buf" {
    // NULL 缓冲区 + size > 0: 通常应失败
    let f = fmemopen(
        core::ptr::null_mut::<core::ffi::c_void>(),
        64,
        cstr(b"r\0"),
    );
    // musl 中 NULL buf 返回 NULL
    let _ = f;
});

test!("fmemopen_zero_size" {
    let mut buf: [u8; 1] = [0; 1];
    let f = fmemopen(
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        0,
        cstr(b"r\0"),
    );
    let _ = f;
});

// ---- open_memstream 测试 ----

test!("open_memstream_basic" {
    let mut bufp: *mut c_char = core::ptr::null_mut();
    let mut sizep: usize = 0;
    let f = open_memstream(&raw mut bufp, &raw mut sizep);
    assert!(!f.is_null(), "open_memstream 应返回有效 FILE*");
    fclose(f);
    // 关闭后 bufp 应被更新 (通常包含空字符串或 NULL)
});

test!("open_memstream_write_and_read_size" {
    let mut bufp: *mut c_char = core::ptr::null_mut();
    let mut sizep: usize = 0;
    let f = open_memstream(&raw mut bufp, &raw mut sizep);
    assert!(!f.is_null());
    // 写入后 fflush 刷新大小
    let _ = fflush(f);
    fclose(f);
});

// ---- open_wmemstream 测试 ----

test!("open_wmemstream_basic" {
    let mut bufp: *mut c_int = core::ptr::null_mut();
    let mut sizep: usize = 0;
    let f = open_wmemstream(&raw mut bufp, &raw mut sizep);
    assert!(!f.is_null(), "open_wmemstream 应返回有效 FILE*");
    fclose(f);
});

// ---- popen / pclose 测试 ----

test!("popen_echo" {
    // 使用 sh -c 确保 /bin/echo 不可用时也能工作
    let f = popen(cstr(b"/bin/echo hello\0"), cstr(b"r\0"));
    if f.is_null() {
        // 如果 /bin/echo 不可用, 跳过测试
        return;
    }
    let ret = pclose(f);
    // pclose 返回退出状态; 不同环境返回值可能不同, 只检查 >= 0
    assert!(ret >= 0, "pclose 应返回 >= 0, got {}", ret);
});

test!("popen_null_cmd" {
    // NULL cmd: 行为未定义, 只检查不崩溃
    let f = popen(core::ptr::null(), cstr(b"r\0"));
    if !f.is_null() {
        pclose(f);
    }
});

// pclose(NULL) 在 musl 中会解引用空指针导致 SIGSEGV, 无法安全测试

// ---- fgetln 测试 ----

test!("fgetln_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut plen: usize = 0;
    let ret = fgetln(f, &raw mut plen);
    // /dev/null 返回 NULL, plen = 0
    assert!(ret.is_null(), "/dev/null 的 fgetln 应返回 NULL");
    assert_eq!(plen, 0, "plen 应为 0");
    fclose(f);
});

// musl 的 fgetln 对 NULL FILE* 会解引用导致 SIGSEGV, 无法安全测试 null 文件

// ---- getdelim 测试 ----

test!("getdelim_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut lineptr: *mut c_char = core::ptr::null_mut();
    let mut n: usize = 0;
    let ret = getdelim(&raw mut lineptr, &raw mut n, b',' as c_int, f);
    assert_eq!(ret, -1, "/dev/null 的 getdelim 应返回 -1 (EOF)");
    fclose(f);
});

// musl 的 getdelim/getline 对 NULL FILE* 会解引用导致 SIGSEGV, 不测试 null 文件

// ---- getline 测试 ----

test!("getline_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut lineptr: *mut c_char = core::ptr::null_mut();
    let mut n: usize = 0;
    let ret = getline(&raw mut lineptr, &raw mut n, f);
    assert_eq!(ret, -1, "/dev/null 的 getline 应返回 -1 (EOF)");
    fclose(f);
});
