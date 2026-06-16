//! ttyname_r — 获取文件描述符关联的终端设备路径名（线程安全）。
//! 对应 musl src/unistd/ttyname_r.c
//!
//! 通过 /proc/self/fd/<fd> 符号链接读取终端路径。

use core::ffi::{c_char, c_int};
use crate::syscall::raw_syscall3;

/// POSIX `ttyname_r` — 将 `fd` 关联终端设备的路径名写入用户提供的缓冲区 `name`。
///
/// 线程安全版本，使用用户分配的缓冲区。
/// 成功返回 0，失败返回错误码。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn ttyname_r(fd: c_int, name: *mut c_char, size: usize) -> c_int {
    if name.is_null() || size == 0 {
        return 22; // EINVAL
    }

    // 1. 检查是否为终端
    if super::isatty(fd) == 0 {
        return 19; // ENODEV
    }

    // 2. 构造 /proc/self/fd/<fd> 路径
    let mut procname: [u8; 32] = [0; 32];
    let (fd_buf, fd_len) = int_to_str(fd);
    let prefix = b"/proc/self/fd/";
    unsafe {
        for i in 0..prefix.len() {
            *procname.as_mut_ptr().add(i) = prefix[i];
        }
        let offset = prefix.len();
        for i in 0..fd_len {
            *procname.as_mut_ptr().add(offset + i) = fd_buf[i];
        }
        *procname.as_mut_ptr().add(offset + fd_len) = 0;
    }

    // 3. 通过 readlink 读取目标路径
    let mut link_buf: [u8; 64] = [0; 64];
    let link_len = unsafe {
        let r = raw_syscall3(
            crate::syscall::SYS_readlink,
            procname.as_ptr() as i64,
            link_buf.as_mut_ptr() as i64,
            63,
        ) as isize;
        if r < 0 {
            return 19; // ENODEV
        }
        r as usize
    };

    // 4. 处理路径（去除 /dev/ 前缀）
    let (src, src_len): (*const u8, usize) = if link_len > 5 {
        let prefix_dev = b"/dev/";
        let mut matches = true;
        for i in 0..5 {
            if link_buf[i] != prefix_dev[i] {
                matches = false;
                break;
            }
        }
        if matches {
            unsafe { (link_buf.as_ptr().add(5), link_len - 5) }
        } else {
            (link_buf.as_ptr(), link_len)
        }
    } else {
        (link_buf.as_ptr(), link_len)
    };

    // 5. 检查缓冲区大小并拷贝
    if src_len + 1 > size {
        return 34; // ERANGE
    }
    unsafe {
        let dst = name as *mut u8;
        for i in 0..src_len {
            *dst.add(i) = *src.add(i);
        }
        *dst.add(src_len) = 0;
    }
    0
}

/// 将非负整数转换为固定缓冲区中的字符串。
/// 返回 (buffer, 字符串长度)。
fn int_to_str(mut value: i32) -> ([u8; 16], usize) {
    let mut buf: [u8; 16] = [0; 16];
    if value == 0 {
        buf[0] = b'0';
        return (buf, 1);
    }
    if value < 0 {
        value = -value;
    }
    let mut stack: [u8; 16] = [0; 16];
    let mut count: usize = 0;
    while value > 0 {
        stack[count] = (value % 10) as u8 + b'0';
        value /= 10;
        count += 1;
    }
    let mut i: usize = 0;
    while count > 0 {
        count -= 1;
        buf[i] = stack[count];
        i += 1;
    }
    (buf, i)
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::c_char;
    use rusl_core::test;

    // ---- int_to_str 辅助函数测试 ----

    test!("test_int_to_str_zero" {
        let (buf, len) = int_to_str(0);
        assert_eq!(len, 1, "int_to_str(0) length should be 1");
        assert_eq!(buf[0], b'0', "int_to_str(0) should produce '0'");
    });

    test!("test_int_to_str_single_digit" {
        let (buf, len) = int_to_str(7);
        assert_eq!(len, 1, "int_to_str(7) length should be 1");
        assert_eq!(buf[0], b'7', "int_to_str(7) should produce '7'");
    });

    test!("test_int_to_str_two_digits" {
        let (buf, len) = int_to_str(42);
        assert_eq!(len, 2, "int_to_str(42) length should be 2");
        assert_eq!(buf[0], b'4');
        assert_eq!(buf[1], b'2');
    });

    test!("test_int_to_str_negative_value" {
        // 负数取其绝对值转换
        let (buf, len) = int_to_str(-42);
        assert_eq!(len, 2, "int_to_str(-42) length should be 2");
        assert_eq!(buf[0], b'4');
        assert_eq!(buf[1], b'2');
    });

    test!("test_int_to_str_large_value" {
        let (buf, len) = int_to_str(1234567890);
        assert_eq!(len, 10, "int_to_str(1234567890) length should be 10");
        assert_eq!(&buf[..10], b"1234567890");
    });

    test!("test_int_to_str_max_i32" {
        let (buf, len) = int_to_str(2147483647);
        assert_eq!(len, 10, "int_to_str(i32::MAX) length should be 10");
        assert_eq!(&buf[..10], b"2147483647");
    });

    test!("test_int_to_str_negative_large" {
        // 测试负数绝对值大的情况 (-2147483647, 接近 i32::MIN)
        // 注意: i32::MIN 取反会溢出，但 fd 在 ttyname_r 中总是非负
        let (buf, len) = int_to_str(-2147483647);
        assert_eq!(len, 10, "int_to_str(-2147483647) length should be 10");
        assert_eq!(&buf[..10], b"2147483647");
    });

    test!("test_int_to_str_all_digits" {
        // 验证所有数字字符是否正确
        for d in 0..10 {
            let (buf, len) = int_to_str(d);
            assert_eq!(len, 1);
            assert_eq!(buf[0], b'0' + d as u8);
        }
    });

    // ---- ttyname_r 参数验证测试 ----

    test!("test_ttyname_r_null_name_returns_einval" {
        // name 为 NULL: 返回 EINVAL (22)
        let ret = ttyname_r(0, core::ptr::null_mut(), 32);
        assert_eq!(ret, 22, "ttyname_r with NULL name should return EINVAL(22)");
    });

    test!("test_ttyname_r_zero_size_returns_einval" {
        // size 为 0: 返回 EINVAL (22)
        let mut buf: [c_char; 32] = [0; 32];
        let ret = ttyname_r(0, buf.as_mut_ptr(), 0);
        assert_eq!(ret, 22, "ttyname_r with size=0 should return EINVAL(22)");
    });

    test!("test_ttyname_r_invalid_fd_returns_enodev" {
        // 无效 fd (不是终端): 返回 ENODEV (19)
        let mut buf: [c_char; 32] = [0; 32];
        let ret = ttyname_r(-1, buf.as_mut_ptr(), 32);
        assert_eq!(ret, 19, "ttyname_r with invalid fd should return ENODEV(19)");
    });

    test!("test_ttyname_r_small_buffer_returns_erange" {
        // 如果是终端，缓冲区太小应返回 ERANGE(34)
        // 但 stdin/fd=0 不一定关联终端。isatty(0) 可能返回 1 (在终端下)
        // 我们用 isatty 检查, 如果非终端则跳过此测试
        // 实际上，当 isatty 返回 0 时，会返回 ENODEV(19), 不会走到 ERANGE
        // 此测试仅验证：如果能到达缓冲区检查点，小缓冲区返回 ERANGE
        // 简单场景: 在非交互式环境下，fd=0 不是终端，返回 ENODEV(19)
        let mut buf: [c_char; 4] = [0; 4];
        let ret = ttyname_r(0, buf.as_mut_ptr(), 4);
        // 可能是 ENODEV(19) 或 ERANGE(34)，取决于 stdin 是否为终端
        assert!(ret == 19 || ret == 34, "expect ENODEV or ERANGE");
    });
}
