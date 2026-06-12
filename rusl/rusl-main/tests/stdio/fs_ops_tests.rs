//! remove / rename / tmpfile / tmpnam / tempnam 集成测试

use core::ffi::c_char;
use super::imports::{remove, rename, tmpfile, tmpnam, tempnam, fclose};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- remove 测试 ----

test!("remove_null_path" {
    let ret = remove(core::ptr::null());
    assert_eq!(ret, -1, "remove(NULL) 应返回 -1");
});

test!("remove_nonexistent" {
    let ret = remove(cstr(b"/tmp/__rusl_nonexistent_remove__.txt\0"));
    assert_eq!(ret, -1, "删除不存在的文件应返回 -1");
});

test!("remove_existing_file" {
    // 创建文件后删除
    let path = b"/tmp/__rusl_test_remove__.txt\0";
    // 先用 fopen 创建
    let f = super::imports::fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    fclose(f);

    // 删除文件
    let ret = remove(cstr(path));
    assert_eq!(ret, 0, "删除刚创建的文件应返回 0");
});

// ---- rename 测试 ----

test!("rename_null_old" {
    let ret = rename(
        core::ptr::null(),
        cstr(b"/tmp/new\0"),
    );
    assert_eq!(ret, -1, "rename(NULL old) 应返回 -1");
});

test!("rename_null_new" {
    let ret = rename(
        cstr(b"/tmp/old\0"),
        core::ptr::null(),
    );
    assert_eq!(ret, -1, "rename(NULL new) 应返回 -1");
});

test!("rename_simple" {
    let old = b"/tmp/__rusl_test_rename_old__.txt\0";
    let new = b"/tmp/__rusl_test_rename_new__.txt\0";

    // 创建源文件
    let f = super::imports::fopen(cstr(old), cstr(b"w\0"));
    assert!(!f.is_null());
    fclose(f);

    // 先删除可能存在的目标文件
    let _ = remove(cstr(new));

    // 重命名
    let ret = rename(cstr(old), cstr(new));
    assert_eq!(ret, 0, "rename 应返回 0");

    // 清理
    let _ = remove(cstr(new));
});

// ---- tmpfile 测试 ----

test!("tmpfile_returns_non_null" {
    // 前置: 调用 tmpfile
    // 后置: 返回非 NULL 的临时文件流
    let f = tmpfile();
    assert!(!f.is_null(), "tmpfile 应返回有效 FILE*");
    // 关闭临时文件会自动删除
    fclose(f);
});

test!("tmpfile_multiple" {
    let f1 = tmpfile();
    let f2 = tmpfile();
    assert!(!f1.is_null());
    assert!(!f2.is_null());
    assert_ne!(f1, f2, "两次 tmpfile 应返回不同的 FILE*");
    fclose(f1);
    fclose(f2);
});

// ---- tmpnam 测试 ----

test!("tmpnam_null_buf" {
    // NULL 缓冲区: tmpnam 使用内部静态缓冲区
    let ret = tmpnam(core::ptr::null_mut());
    assert!(!ret.is_null(), "tmpnam(NULL) 应返回非 NULL 字符串");
});

test!("tmpnam_with_buf" {
    let mut buf: [u8; 64] = [0; 64];
    let ret = tmpnam(buf.as_mut_ptr() as *mut c_char);
    assert!(!ret.is_null(), "tmpnam(buf) 应返回非 NULL");
    // 返回值应等于 buf 地址
    assert_eq!(
        ret as usize,
        buf.as_mut_ptr() as usize,
        "tmpnam 应返回与传入相同的指针"
    );
});

// ---- tempnam 测试 ----

test!("tempnam_null_dir" {
    let ret = tempnam(
        core::ptr::null(),
        cstr(b"pfx\0"),
    );
    assert!(!ret.is_null(), "tempnam(NULL dir) 应返回非 NULL");
    // tempnam 分配内存, 需要 free(), 这里允许泄漏
});

test!("tempnam_null_pfx" {
    let ret = tempnam(
        cstr(b"/tmp\0"),
        core::ptr::null(),
    );
    assert!(!ret.is_null(), "tempnam(NULL pfx) 应返回非 NULL");
});

test!("tempnam_both_null" {
    let ret = tempnam(core::ptr::null(), core::ptr::null());
    assert!(!ret.is_null(), "tempnam(NULL, NULL) 应返回非 NULL");
});
