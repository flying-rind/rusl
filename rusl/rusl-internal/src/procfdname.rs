//! procfdname 模块 — 构造 `/proc/self/fd/N` 路径字符串。
//!
//! 本模块是 rusl 内部辅助函数，用于将文件描述符号转换为对应的
//! Linux procfs 路径，供 `fstat`、`fchdir` 等通过 `/proc` 实现
//! fd 操作的模块使用。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

use core::ffi::c_uint;

/// 构造 `/proc/self/fd/N` 路径字符串。
///
/// 将文件描述符 `fd` 转换为 `/proc/self/fd/<fd>` 格式的 NUL 终止
/// 路径字符串并写入 `buf`。
///
/// # 参数
///
/// * `buf` - 输出缓冲区，调用者确保长度 >= `15 + 3 * sizeof(c_int)`。
/// * `fd` - 文件描述符号（非负整数）或 `AT_FDCWD` 特殊值。
///
/// # 前置条件
///
/// * `buf.len() >= 15 + 3 * core::mem::size_of::<c_int>()`
/// * `fd` 为有效的文件描述符编号或 `AT_FDCWD`
///
/// # 后置条件
///
/// * `buf` 中写入 `/proc/self/fd/N` 格式的 NUL 终止字符串
/// * 路径长度为 `15 + floor(log10(fd))` 字节（不含 NUL）
/// * `fd == 0` 时写入 `/proc/self/fd/0`（含 NUL 共 15 字节）
///
/// # 系统算法
///
/// 使用两遍扫描策略避免反转或临时缓冲区：
/// 1. 复制前缀 `/proc/self/fd/` 到 `buf`
/// 2. 若 `fd == 0`，直接写入 `'0'` 和 NUL 终止符
/// 3. 第一遍：测量 `fd` 的十进制位数
/// 4. NUL 预置：在最终位置写入终止符
/// 5. 第二遍：从右向左反向填充数字字符
pub fn procfdname(buf: &mut [u8], fd: c_uint) {
    let prefix = b"/proc/self/fd/";
    let prefix_len = prefix.len();

    // 第一步：复制前缀到缓冲区
    for (i, &ch) in prefix.iter().enumerate() {
        buf[i] = ch;
    }

    // 第二步：处理 fd == 0 的特殊情况
    if fd == 0 {
        buf[prefix_len] = b'0';
        buf[prefix_len + 1] = b'\0';
        return;
    }

    // 第三步：第一遍扫描 — 测量 fd 的十进制位数
    let mut j = fd;
    let mut digits: usize = 0;
    while j != 0 {
        j /= 10;
        digits += 1;
    }

    let end = prefix_len + digits;

    // 第四步：NUL 预置 — 在最终位置写入终止符
    buf[end] = b'\0';

    // 第五步：第二遍扫描 — 从右向左反向填充数字字符
    let mut n = fd;
    let mut pos = end;
    while n != 0 {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
    }
}

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::procfdname;

    /// 辅助函数：从 buf 中提取 NUL 终止的字符串
    fn buf_to_str(buf: &[u8]) -> &str {
        let end = buf.iter().position(|&b| b == b'\0').expect("buffer must be NUL-terminated");
        core::str::from_utf8(&buf[..end]).expect("valid UTF-8")
    }

    test!("procfdname_fd0" {
        let mut buf = [0u8; 32];
        procfdname(&mut buf, 0);
        assert_eq!(buf_to_str(&buf), "/proc/self/fd/0");
    });

    test!("procfdname_fd1" {
        let mut buf = [0u8; 32];
        procfdname(&mut buf, 1);
        assert_eq!(buf_to_str(&buf), "/proc/self/fd/1");
    });

    test!("procfdname_fd10" {
        let mut buf = [0u8; 32];
        procfdname(&mut buf, 10);
        assert_eq!(buf_to_str(&buf), "/proc/self/fd/10");
    });

    test!("procfdname_fd100" {
        let mut buf = [0u8; 32];
        procfdname(&mut buf, 100);
        assert_eq!(buf_to_str(&buf), "/proc/self/fd/100");
    });

    test!("procfdname_fd999" {
        let mut buf = [0u8; 32];
        procfdname(&mut buf, 999);
        assert_eq!(buf_to_str(&buf), "/proc/self/fd/999");
    });

    test!("procfdname_large_fd" {
        let mut buf = [0u8; 64];
        procfdname(&mut buf, 1234567890);
        assert_eq!(buf_to_str(&buf), "/proc/self/fd/1234567890");
    });

    test!("procfdname_nul_terminated" {
        let mut buf = [0xffu8; 32];
        procfdname(&mut buf, 5);
        let path = buf_to_str(&buf);
        assert_eq!(path, "/proc/self/fd/5");
        // 验证 NUL 后原始字节未被修改（缓冲区中前缀那部分会被覆盖）
        let after_nul = path.len() + 1; // 跳过 NUL
        assert_eq!(buf[after_nul], 0xff);
    });
}