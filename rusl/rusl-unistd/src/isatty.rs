//! isatty — 测试文件描述符是否关联到终端设备。
//! 对应 musl src/unistd/isatty.c
//!
//! 通过 TIOCGWINSZ ioctl 判断。

use core::ffi::c_int;
use rusl_internal::syscall::raw_syscall3;

/// winsize 结构体（用于 TIOCGWINSZ ioctl）。
#[repr(C)]
struct WinSize {
    ws_row: u16,
    ws_col: u16,
    _ws_xpixel: u16,
    _ws_ypixel: u16,
}

/// TIOCGWINSZ — 获取终端窗口大小的 ioctl 命令
const TIOCGWINSZ: i64 = 0x5413;

/// POSIX `isatty` — 测试文件描述符 `fd` 是否关联到一个终端设备。
///
/// 通过尝试对 `fd` 执行 `TIOCGWINSZ` ioctl 来判断：
/// 成功返回 1（是终端），失败返回 0（不是终端）。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn isatty(fd: c_int) -> c_int {
    let _wsz = WinSize {
        ws_row: 0,
        ws_col: 0,
        _ws_xpixel: 0,
        _ws_ypixel: 0,
    };
    let r = unsafe {
        raw_syscall3(
            rusl_internal::syscall::SYS_ioctl,
            fd as i64,
            TIOCGWINSZ,
            &_wsz as *const WinSize as i64,
        )
    };
    if r == 0 { 1 } else { 0 }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_isatty_invalid_fd" {
        // 无效 fd（如 -1）: 返回 0（不是终端）
        let ret = isatty(-1);
        assert_eq!(ret, 0, "isatty(-1) should return 0 (not a tty)");
    });

    test!("test_isatty_very_large_fd" {
        // 非常大的 fd: 返回 0
        let ret = isatty(99999);
        assert_eq!(ret, 0, "isatty(99999) should return 0 (not a tty)");
    });

    test!("test_isatty_pipe_fd" {
        // 管道 fd: 返回 0 (不是终端)
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let result = isatty(fds[0]);
        assert_eq!(result, 0, "isatty on pipe fd should return 0");

        crate::close(fds[0]);
        crate::close(fds[1]);
    });

    test!("test_isatty_tiocgwinsz_constant" {
        // 验证 TIOCGWINSZ 常量
        assert_eq!(TIOCGWINSZ, 0x5413, "TIOCGWINSZ should be 0x5413");
    });

    test!("test_isatty_std_fds" {
        // 测试标准文件描述符（stdin, stdout, stderr）
        // 在交互式终端下可能返回 1，在管道/重定向下返回 0
        // 只验证函数不崩溃且返回 0 或 1
        let r0 = isatty(0);
        let r1 = isatty(1);
        let r2 = isatty(2);
        assert!(r0 == 0 || r0 == 1, "isatty(0) should return 0 or 1");
        assert!(r1 == 0 || r1 == 1, "isatty(1) should return 0 or 1");
        assert!(r2 == 0 || r2 == 1, "isatty(2) should return 0 or 1");
    });
}
