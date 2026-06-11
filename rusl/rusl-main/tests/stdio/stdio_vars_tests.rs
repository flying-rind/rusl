//! stdin / stdout / stderr / setbuf / setvbuf 集成测试

use core::ffi::c_char;
use super::imports::{
    stdin, stdout, stderr,
    fopen, fclose, fileno, feof, ferror,
    setbuf, setvbuf,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// -----------------------------------------------------------------------
// 标准流全局变量测试
// -----------------------------------------------------------------------

test!("stdin_not_null" {
    // 前置: 程序启动后 stdin 由 CRT 初始化
    // 后置: 指针非空
    assert!(!unsafe { stdin }.is_null(), "stdin 不应为 NULL");
});

test!("stdout_not_null" {
    assert!(!unsafe { stdout }.is_null(), "stdout 不应为 NULL");
});

test!("stderr_not_null" {
    assert!(!unsafe { stderr }.is_null(), "stderr 不应为 NULL");
});

test!("stdin_fileno_is_zero" {
    // 前置: stdin 对应 fd 0
    // 后置: fileno(stdin) == 0
    let fd = fileno(unsafe { stdin });
    assert_eq!(fd, 0, "stdin fd 应为 0");
});

test!("stdout_fileno_is_one" {
    // 前置: stdout 对应 fd 1
    // 后置: fileno(stdout) == 1
    let fd = fileno(unsafe { stdout });
    assert_eq!(fd, 1, "stdout fd 应为 1");
});

test!("stderr_fileno_is_two" {
    // 前置: stderr 对应 fd 2
    // 后置: fileno(stderr) == 2
    let fd = fileno(unsafe { stderr });
    assert_eq!(fd, 2, "stderr fd 应为 2");
});

test!("stdin_no_error_initially" {
    // 前置: 标准流初始化后不应有错误标志
    // 后置: feof/ferror 均为 0
    let s = unsafe { stdin };
    assert_eq!(feof(s), 0, "stdin 初始 feof 应为 0");
    assert_eq!(ferror(s), 0, "stdin 初始 ferror 应为 0");
});

test!("stdout_no_error_initially" {
    let s = unsafe { stdout };
    assert_eq!(feof(s), 0, "stdout 初始 feof 应为 0");
    assert_eq!(ferror(s), 0, "stdout 初始 ferror 应为 0");
});

test!("stderr_no_error_initially" {
    let s = unsafe { stderr };
    assert_eq!(feof(s), 0, "stderr 初始 feof 应为 0");
    assert_eq!(ferror(s), 0, "stderr 初始 ferror 应为 0");
});

test!("stdin_stdout_stderr_distinct" {
    // 前置: 三个标准流应指向不同对象
    // 后置: 指针互不相等
    unsafe {
        assert_ne!(stdin, stdout, "stdin != stdout");
        assert_ne!(stdin, stderr, "stdin != stderr");
        assert_ne!(stdout, stderr, "stdout != stderr");
    }
});

// -----------------------------------------------------------------------
// setbuf 测试
// -----------------------------------------------------------------------

test!("setbuf_null_buffer" {
    // 前置: buf 为 NULL（无缓冲模式）
    // 后置: 不应崩溃
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    setbuf(f, core::ptr::null_mut());
    fclose(f);
});

test!("setbuf_with_buffer" {
    // 前置: 提供用户缓冲区
    // 后置: 不应崩溃
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 256] = [0; 256];
    setbuf(f, buf.as_mut_ptr() as *mut c_char);
    fclose(f);
});

// -----------------------------------------------------------------------
// setvbuf 测试
// -----------------------------------------------------------------------

test!("setvbuf_null_file" {
    // 前置: NULL FILE*
    // 后置: 返回 -1
    let mut buf: [u8; 256] = [0; 256];
    let ret = setvbuf(core::ptr::null_mut(), buf.as_mut_ptr() as *mut c_char, 0 /* _IOFBF */, 256);
    assert_eq!(ret, -1, "setvbuf(NULL) 应返回 -1");
});

test!("setvbuf_ionbf" {
    // 前置: 无缓冲模式 _IONBF = 2
    // 后置: 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = setvbuf(f, core::ptr::null_mut(), 2 /* _IONBF */, 0);
    assert_eq!(ret, 0, "setvbuf(_IONBF) 应返回 0");
    fclose(f);
});

test!("setvbuf_iofbf" {
    // 前置: 全缓冲模式 _IOFBF = 0
    // 后置: 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 512] = [0; 512];
    let ret = setvbuf(f, buf.as_mut_ptr() as *mut c_char, 0 /* _IOFBF */, 512);
    assert_eq!(ret, 0, "setvbuf(_IOFBF) 应返回 0");
    fclose(f);
});

test!("setvbuf_iolbf" {
    // 前置: 行缓冲模式 _IOLBF = 1
    // 后置: 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 256] = [0; 256];
    let ret = setvbuf(f, buf.as_mut_ptr() as *mut c_char, 1 /* _IOLBF */, 256);
    assert_eq!(ret, 0, "setvbuf(_IOLBF) 应返回 0");
    fclose(f);
});

test!("setvbuf_size_zero" {
    // 前置: size = 0
    // 后置: 返回 0（musl 对待 size=0 等同无缓冲）
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = setvbuf(f, core::ptr::null_mut(), 0 /* _IOFBF */, 0);
    assert_eq!(ret, 0);
    fclose(f);
});
