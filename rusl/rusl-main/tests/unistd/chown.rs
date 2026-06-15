//! chown 函数集成测试

use core::ffi::c_char;
use test_framework::test;

// (uid_t)-1 表示不改变所有者
const NOCHANGE: u32 = !0u32;

test!("test_chown_nochange" {
    {
        // uid=-1, gid=-1 表示不改变任何东西，应成功
        let path = b"/dev/null\0";
        let ret = super::chown(
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
        );
        // 由于我们没有权限改变 /dev/null 的所有者，
        // 用 -1/-1 表示不改变，这应该成功
        assert_eq!(ret, 0);
    }
});

test!("test_chown_noent" {
    {
        let path = b"/nonexistent_file_rusl_test_xyz\0";
        let ret = super::chown(
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_chown_root_file_no_change" {
    {
        // /etc/hostname or similar root-owned file
        let path = b"/\0";
        let ret = super::chown(
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
        );
        // chown with -1,-1 on root directory should succeed
        assert_eq!(ret, 0);
    }
});
