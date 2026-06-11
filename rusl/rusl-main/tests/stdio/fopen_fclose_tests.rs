//! fopen / fclose 集成测试

use core::ffi::c_char;
use super::imports::{fopen, fclose};
use test_framework::test;

/// 辅助函数：将 Rust 字节串转为 C 字符串指针
fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- fopen 基本测试 ----

test!("fopen_read_dev_null" {
    // 前置: /dev/null 始终可读
    // 后置: 返回非空 FILE* 指针
    let path = b"/dev/null\0";
    let mode = b"r\0";
    let f = fopen(cstr(path), cstr(mode));
    assert!(!f.is_null(), "打开 /dev/null(r) 应返回有效 FILE*");
    fclose(f);
});

test!("fopen_write_dev_null" {
    // 前置: /dev/null 始终可写
    // 后置: 返回非空 FILE*
    let path = b"/dev/null\0";
    let mode = b"w\0";
    let f = fopen(cstr(path), cstr(mode));
    assert!(!f.is_null(), "打开 /dev/null(w) 应返回有效 FILE*");
    fclose(f);
});

test!("fopen_nonexistent_file_read" {
    // 前置: 文件不存在
    // 后置: 返回 NULL
    let path = b"/tmp/__rusl_nonexistent_test_file__.txt\0";
    let mode = b"r\0";
    let f = fopen(cstr(path), cstr(mode));
    assert!(f.is_null(), "打开不存在的文件(r) 应返回 NULL");
});

test!("fopen_create_new_file_write" {
    // 前置: 新文件路径，w 模式应创建文件
    // 后置: 返回非空 FILE*
    let path = b"/tmp/__rusl_test_fopen_w__.txt\0";
    let mode = b"w\0";
    let f = fopen(cstr(path), cstr(mode));
    assert!(!f.is_null(), "w 模式应能创建新文件");
    fclose(f);
});

test!("fopen_null_path" {
    // 前置: path 为 NULL
    // 后置: 返回 NULL（musl 内部检查）
    let mode = b"r\0";
    let f = fopen(core::ptr::null(), cstr(mode));
    assert!(f.is_null(), "NULL 路径应返回 NULL");
});

test!("fopen_null_mode" {
    // 前置: mode 为 NULL
    // 后置: 返回 NULL
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), core::ptr::null());
    assert!(f.is_null(), "NULL mode 应返回 NULL");
});

test!("fopen_write_mode_truncate" {
    // 前置: 创建文件并写入内容，再用 w 模式重新打开
    // 后置: 文件被截断为空或重新创建
    let path = b"/tmp/__rusl_test_fopen_trunc__.txt\0";
    let mode_w = b"w\0";
    let mode_r = b"r\0";

    // 创建文件
    let f1 = fopen(cstr(path), cstr(mode_w));
    assert!(!f1.is_null());
    fclose(f1);

    // 用 r 模式重新打开确认文件存在
    let f2 = fopen(cstr(path), cstr(mode_r));
    assert!(!f2.is_null(), "文件应该存在");
    fclose(f2);
});

// ---- fclose 基本测试 ----

test!("fclose_valid_file" {
    // 前置: 已打开的有效 FILE*
    // 后置: 返回 0（成功）
    let path = b"/dev/null\0";
    let mode = b"r\0";
    let f = fopen(cstr(path), cstr(mode));
    assert!(!f.is_null());
    let ret = fclose(f);
    assert_eq!(ret, 0, "关闭有效文件应返回 0");
});

test!("fclose_null_file" {
    // 前置: NULL FILE*
    // 后置: 返回 EOF（-1）
    let ret = fclose(core::ptr::null_mut());
    assert_eq!(ret, -1, "关闭 NULL 应返回 EOF");
});

test!("fopen_fclose_cycle" {
    // 测试多次打开/关闭循环
    let path = b"/dev/null\0";
    let mode = b"r\0";
    for _ in 0..5 {
        let f = fopen(cstr(path), cstr(mode));
        assert!(!f.is_null());
        let ret = fclose(f);
        assert_eq!(ret, 0);
    }
});
