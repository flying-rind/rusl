//! dup3 函数集成测试

use core::ffi::c_void;
use test_framework::test;

// Linux O_CLOEXEC 标志值 (02000000 in C octal = 524288)
const O_CLOEXEC: super::c_int = 0o2000000;

test!("test_dup3_same_fd" {
        // dup3 错误处理: old == new (EINVAL)
    {
        // dup3 要求 old != new
        let ret = super::dup3(super::STDOUT_FILENO, super::STDOUT_FILENO, 0);
        assert_eq!(ret, -1, "dup3 with old==new should return -1 (EINVAL)");
    }
});

test!("test_dup3_invalid_old" {
        // dup3 错误处理: 无效 old fd
    {
        let ret = super::dup3(-1, 10, 0);
        assert_eq!(ret, -1, "dup3 with invalid old should return -1 (EBADF)");
    }
});

test!("test_dup3_basic" {
        // dup3 基本功能: 复制 fd, flags=0
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 先 dup 获得一个空闲 fd 作为目标
        let target = super::dup(fds[0]);
        assert!(target >= 0, "dup should succeed");

        // dup3 到同一个 fd
        let ret2 = super::dup3(fds[0], target, 0);
        assert_eq!(ret2, target, "dup3 with flags=0 should return target fd");

        // 通过目标 fd 读取验证
        let data: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        super::write(fds[1], data.as_ptr() as *const c_void, 4);

        let mut buf: [u8; 8] = [0; 8];
        let r = super::read(target, buf.as_mut_ptr() as *mut c_void, 8);
        assert_eq!(r, 4, "read via dup3'd fd should succeed");
        assert_eq!(&buf[..4], &data[..], "data should match");

        super::close(target);
        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_dup3_cloexec" {
        // dup3 基本功能: 使用 O_CLOEXEC 标志
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 先 dup 获得一个空闲 fd
        let target = super::dup(fds[0]);
        assert!(target >= 0, "dup should succeed");

        // dup3 使用 O_CLOEXEC 标志
        let ret2 = super::dup3(fds[0], target, O_CLOEXEC);
        assert_eq!(ret2, target, "dup3 with O_CLOEXEC should return target fd");

        // 通过目标 fd 读取验证功能正常
        let data: [u8; 3] = [1, 2, 3];
        super::write(fds[1], data.as_ptr() as *const c_void, 3);

        let mut buf: [u8; 4] = [0; 4];
        let r = super::read(target, buf.as_mut_ptr() as *mut c_void, 4);
        assert_eq!(r, 3, "should read 3 bytes via O_CLOEXEC dup3");

        super::close(target);
        super::close(fds[0]);
        super::close(fds[1]);
    }
});
