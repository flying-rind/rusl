//! aio 模块对外接口 — 异步 I/O (POSIX AIO)
//!
//! 当禁用 `rusl` feature 时，通过 extern "C" 声明 musl libc.a 中的符号；
//! 启用 `rusl` feature 时，各符号由 `rusl-aio` crate 提供。

use core::ffi::{c_int, c_void};

// ============================================================================
// 类型定义
// ============================================================================

/// POSIX 异步 I/O 控制块 (`struct aiocb`)
#[repr(C)]
pub struct aiocb {
    pub aio_fildes: c_int,
    pub aio_lio_opcode: c_int,
    pub aio_reqprio: c_int,
    pub aio_buf: *mut c_void,
    pub aio_nbytes: usize,
    // aio_sigevent — 不透明占位 (实际布局由 sigevent 定义)
    pub aio_sigevent_opaque: [u8; 64],
    pub __pad: [c_int; 1],
    pub __next: *mut aiocb,
    pub __prev: *mut aiocb,
    pub __td: *mut c_void,
    pub __ss: isize,
    pub __err: c_int,
}

// ============================================================================
// 常量
// ============================================================================

pub const AIO_CANCELED: c_int = 0;
pub const AIO_NOTCANCELED: c_int = 1;
pub const AIO_ALLDONE: c_int = 2;
pub const LIO_READ: c_int = 0;
pub const LIO_WRITE: c_int = 1;
pub const LIO_NOP: c_int = 2;
pub const LIO_WAIT: c_int = 0;
pub const LIO_NOWAIT: c_int = 1;

// ============================================================================
// 内部 FFI 声明
// ============================================================================

extern "C" {
    #[link_name = "aio_read"]
    fn musl_aio_read(cb: *mut aiocb) -> c_int;
    #[link_name = "aio_write"]
    fn musl_aio_write(cb: *mut aiocb) -> c_int;
    #[link_name = "aio_fsync"]
    fn musl_aio_fsync(op: c_int, cb: *mut aiocb) -> c_int;
    #[link_name = "aio_return"]
    fn musl_aio_return(cb: *mut aiocb) -> isize;
    #[link_name = "aio_error"]
    fn musl_aio_error(cb: *const aiocb) -> c_int;
    #[link_name = "aio_cancel"]
    fn musl_aio_cancel(fd: c_int, cb: *mut aiocb) -> c_int;
    #[link_name = "aio_suspend"]
    fn musl_aio_suspend(cbs: *const *const aiocb, cnt: c_int, ts: *const ()) -> c_int;
    #[link_name = "lio_listio"]
    fn musl_lio_listio(mode: c_int, cbs: *const *mut aiocb, cnt: c_int, sev: *mut c_void) -> c_int;
}

// ============================================================================
// safe 公共封装
// ============================================================================

/// 提交异步读请求
pub extern "C" fn aio_read(cb: *mut aiocb) -> c_int {
    unsafe { musl_aio_read(cb) }
}

/// 提交异步写请求
pub extern "C" fn aio_write(cb: *mut aiocb) -> c_int {
    unsafe { musl_aio_write(cb) }
}

/// 提交异步 fsync 请求
pub extern "C" fn aio_fsync(op: c_int, cb: *mut aiocb) -> c_int {
    unsafe { musl_aio_fsync(op, cb) }
}

/// 获取已完成的异步操作的返回值
pub extern "C" fn aio_return(cb: *mut aiocb) -> isize {
    unsafe { musl_aio_return(cb) }
}

/// 查询异步操作状态
pub extern "C" fn aio_error(cb: *const aiocb) -> c_int {
    unsafe { musl_aio_error(cb) }
}

/// 取消文件描述符上的异步 I/O 操作
pub extern "C" fn aio_cancel(fd: c_int, cb: *mut aiocb) -> c_int {
    unsafe { musl_aio_cancel(fd, cb) }
}

/// 挂起调用线程直到异步 I/O 操作完成
pub extern "C" fn aio_suspend(cbs: *const *const aiocb, cnt: c_int, ts: *const ()) -> c_int {
    unsafe { musl_aio_suspend(cbs, cnt, ts) }
}

/// 批量提交异步 I/O 请求列表
pub extern "C" fn lio_listio(mode: c_int, cbs: *const *mut aiocb, cnt: c_int, sev: *mut c_void) -> c_int {
    unsafe { musl_lio_listio(mode, cbs, cnt, sev) }
}
