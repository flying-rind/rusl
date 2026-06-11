//! ext — GNU stdio_ext.h 扩展函数（第一部分）。
//! 对应 musl src/stdio/ext.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// _flushlbf — 刷新所有行缓冲的 FILE 流。
/// 通过调用 fflush(NULL) 刷新所有打开的输出流。
#[no_mangle]
pub extern "C" fn _flushlbf() {
    unimplemented!()
}

/// __fsetlocking — 设置 FILE 流的锁定行为。
/// musl 实现始终返回 0（锁定行为未改变）。
#[no_mangle]
pub extern "C" fn __fsetlocking(_f: *mut FILE, _type_: c_int) -> c_int {
    unimplemented!()
}

/// __fwriting — 查询流是否处于"正在写入"状态。
/// 检查 F_NORD 标志或写缓冲区挂起数据。
#[no_mangle]
pub extern "C" fn __fwriting(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// __freading — 查询流是否处于"正在读取"状态。
/// 检查 F_NOWR 标志或读缓冲区可用数据。
#[no_mangle]
pub extern "C" fn __freading(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// __freadable — 查询流是否可读（F_NORD 未设置）。
#[no_mangle]
pub extern "C" fn __freadable(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// __fwritable — 查询流是否可写（F_NOWR 未设置）。
#[no_mangle]
pub extern "C" fn __fwritable(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// __flbf — 查询流是否使用行缓冲模式（lbf >= 0）。
#[no_mangle]
pub extern "C" fn __flbf(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// __fbufsize — 返回流的缓冲区大小（buf_size 字段）。
#[no_mangle]
pub extern "C" fn __fbufsize(_f: *mut FILE) -> usize {
    unimplemented!()
}

/// __fpending — 返回写缓冲区中待写入的字节数（wpos - wbase）。
#[no_mangle]
pub extern "C" fn __fpending(_f: *mut FILE) -> usize {
    unimplemented!()
}

/// __fpurge — 清空 FILE 流的所有内部缓冲区（读和写），主实现。
/// 将 wpos、wbase、wend、rpos、rend 全部置零。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fpurge(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// fpurge — __fpurge 的弱别名。对外导出，行为与 __fpurge 完全一致。
#[no_mangle]
pub extern "C" fn fpurge(_f: *mut FILE) -> c_int {
    unimplemented!()
}
