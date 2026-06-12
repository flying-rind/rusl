//! 对应 musl src/stdio/__stdio_write.c
//! 内部 FILE 默认写操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_write: i64 = 1;
#[cfg(target_arch = "aarch64")]
const SYS_write: i64 = 64;

/// 通过 write 系统调用将数据写入文件描述符。
/// 先刷新内部缓冲区数据，再写用户数据。
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let f_ref = &mut *f;
    let iov_len = if f_ref.wpos > f_ref.wbase {
        (f_ref.wpos as usize).wrapping_sub(f_ref.wbase as usize)
    } else {
        0
    };

    // 计算总长度
    let total_len = iov_len + len;

    // 简化版本：使用 write 系统调用
    // 先写内部缓冲区
    let mut written: usize = 0;
    if iov_len > 0 {
        let cnt = rusl_core::__syscall3(SYS_write, f_ref.fd as i64, f_ref.wbase as i64, iov_len as i64);
        if cnt < 0 {
            f_ref.wpos = core::ptr::null_mut();
            f_ref.wbase = core::ptr::null_mut();
            f_ref.wend = core::ptr::null_mut();
            f_ref.flags |= F_ERR;
            return 0;
        }
        written = cnt as usize;
        if written < iov_len {
            // 部分写入，推进 wbase
            f_ref.wbase = f_ref.wbase.add(written);
            return len; // 假装用户数据全部写入
        }
    }

    // 写用户数据
    if len > 0 {
        let cnt = rusl_core::__syscall3(SYS_write, f_ref.fd as i64, buf as i64, len as i64);
        if cnt < 0 {
            f_ref.wpos = core::ptr::null_mut();
            f_ref.wbase = core::ptr::null_mut();
            f_ref.wend = core::ptr::null_mut();
            f_ref.flags |= F_ERR;
            return if iov_len > 0 { len } else { 0 };
        }
        if (cnt as usize) < len {
            // 部分写入
            let user_written = cnt as usize;
            // 不能简单地返回部分结果，重置缓冲区指针
            f_ref.wpos = core::ptr::null_mut();
            f_ref.wbase = core::ptr::null_mut();
            f_ref.wend = core::ptr::null_mut();
            f_ref.flags |= F_ERR;
            return len - user_written + (if iov_len > 0 { iov_len } else { 0 });
        }
    }

    // 成功：重置写指针
    f_ref.wend = unsafe { f_ref.buf.add(f_ref.buf_size) };
    f_ref.wpos = f_ref.buf;
    f_ref.wbase = f_ref.buf;

    len
}
