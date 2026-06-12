//! flockfile / ftrylockfile / funlockfile 集成测试
//!
//! 线程安全锁定函数的烟雾测试。

use super::imports::{fopen, fclose, flockfile, ftrylockfile, funlockfile};
use test_framework::test;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}

// ---- flockfile 测试 ----

test!("flockfile_unlockfile" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());

    // 加锁
    flockfile(f);

    // 解锁
    funlockfile(f);

    fclose(f);
});

test!("flockfile_double_lock" {
    // 同一线程重复加锁: 应安全（递归锁）
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());

    flockfile(f);
    flockfile(f); // 再次加锁

    funlockfile(f);
    funlockfile(f); // 对应解锁

    fclose(f);
});

// ---- ftrylockfile 测试 ----

test!("ftrylockfile_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());

    let ret = ftrylockfile(f);
    assert_eq!(ret, 0, "ftrylockfile 应返回 0 (成功获取锁)");

    funlockfile(f);
    fclose(f);
});

test!("ftrylockfile_already_locked" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());

    // 先 flockfile 获取锁
    flockfile(f);

    // 同一线程中 trylock 应成功（递归锁）
    let ret = ftrylockfile(f);
    assert_eq!(ret, 0, "同线程 ftrylockfile 应成功");

    funlockfile(f);
    funlockfile(f);
    fclose(f);
});

// ---- funlockfile 测试 ----

test!("funlockfile_after_flockfile" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());

    flockfile(f);
    funlockfile(f);

    fclose(f);
});
