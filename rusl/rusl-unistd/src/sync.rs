//! sync — 同步所有文件系统缓冲区。
//! 对应 musl src/unistd/sync.c
//!
//! SYS_sync 系统调用的薄封装（无返回值）。

use crate::syscall::raw_syscall0;

/// sync() — 将所有已修改的文件系统缓冲区和元数据提交到磁盘 I/O 队列。
///
/// 调用不等待 I/O 完成即返回。无参数，无返回值。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn sync() {
    unsafe { raw_syscall0(crate::syscall::SYS_sync); }
}
