//! fseek / fseeko — 文件流定位操作。
//! 对应 musl src/stdio/fseek.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_long};
use super::stdio_impl::FILE;

/// off_t 类型（x86_64 上为 c_long = i64）
pub type off_t = c_long;

const SEEK_SET: c_int = 0;
const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;

/// 内部不加锁文件定位引擎。
/// [Visibility]: Internal (hidden) — 由 __fseeko / __fseeko_unlocked 内部调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fseeko_unlocked(f: *mut FILE, off: off_t, whence: c_int) -> c_int {
    unsafe {
        let f_ref = &mut *f;

        // 验证 whence
        if whence != SEEK_CUR && whence != SEEK_SET && whence != SEEK_END {
            return -1;
        }

        // 调整相对偏移（考虑未消费的读缓冲区）
        let mut adjusted_off = off;
        if whence == SEEK_CUR && !f_ref.rend.is_null() {
            adjusted_off -= (f_ref.rend as usize).wrapping_sub(f_ref.rpos as usize) as i64;
        }

        // 刷新写缓冲区
        if f_ref.wpos != f_ref.wbase {
            if let Some(write_fn) = f_ref.write {
                write_fn(f, core::ptr::null(), 0);
            }
            if f_ref.wpos.is_null() {
                return -1;
            }
        }

        // 离开写模式
        f_ref.wpos = core::ptr::null_mut();
        f_ref.wbase = core::ptr::null_mut();
        f_ref.wend = core::ptr::null_mut();

        // 执行底层 seek
        if let Some(seek_fn) = f_ref.seek {
            if seek_fn(f, adjusted_off, whence) < 0 {
                return -1;
            }
        } else {
            return -1;
        }

        // seek 成功后丢弃读缓冲区
        f_ref.rpos = core::ptr::null_mut();
        f_ref.rend = core::ptr::null_mut();
        f_ref.flags &= !super::stdio_impl::F_EOF;

        0
    }
}

/// 内部加锁文件定位（fseeko 的主实现）。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int {
    unsafe { __fseeko_unlocked(f, off, whence) }
}

/// 标准文件定位（c_long 偏移量）。
#[no_mangle]
pub extern "C" fn fseek(f: *mut FILE, off: c_long, whence: c_int) -> c_int {
    unsafe { __fseeko(f, off, whence) }
}

/// POSIX 大文件定位（off_t 偏移量，弱别名 -> __fseeko）。
#[no_mangle]
pub extern "C" fn fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int {
    unsafe { __fseeko(f, off, whence) }
}
