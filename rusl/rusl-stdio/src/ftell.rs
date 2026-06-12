//! ftell / ftello — 文件流当前位置查询。
//! 对应 musl src/stdio/ftell.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_long};
use super::stdio_impl::FILE;

/// off_t 类型（x86_64 上为 c_long = i64）
pub type off_t = c_long;

const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;
const LONG_MAX: i64 = i64::MAX;

/// 内部不加锁位置查询引擎。
#[no_mangle]
pub(crate) unsafe extern "C" fn __ftello_unlocked(f: *mut FILE) -> off_t {
    unsafe {
        let f_ref = &mut *f;

        // 使用 seek(0, SEEK_CUR) or SEEK_END 获取位置
        let whence = if (f_ref.flags & super::stdio_impl::F_APP) != 0
            && f_ref.wpos != f_ref.wbase
        {
            SEEK_END
        } else {
            SEEK_CUR
        };

        let pos = if let Some(seek_fn) = f_ref.seek {
            seek_fn(f, 0, whence)
        } else {
            return -1;
        };

        if pos < 0 {
            return pos;
        }

        let mut result = pos;

        // 调整缓冲区中的数据
        if !f_ref.rend.is_null() {
            // 有读缓冲区未消费数据
            result += (f_ref.rpos as isize).wrapping_sub(f_ref.rend as isize) as i64;
        } else if !f_ref.wbase.is_null() {
            // 有写缓冲区未刷新数据
            result += (f_ref.wpos as isize).wrapping_sub(f_ref.wbase as isize) as i64;
        }

        result
    }
}

/// 内部加锁位置查询（ftello 的主实现）。
#[no_mangle]
pub(crate) unsafe extern "C" fn __ftello(f: *mut FILE) -> off_t {
    unsafe { __ftello_unlocked(f) }
}

/// 标准当前位置查询（c_long 返回值）。
#[no_mangle]
pub extern "C" fn ftell(f: *mut FILE) -> c_long {
    unsafe {
        let pos = __ftello(f);
        if pos > LONG_MAX {
            return -1;
        }
        pos as c_long
    }
}

/// POSIX 大文件位置查询（off_t 返回值）。
#[no_mangle]
pub extern "C" fn ftello(f: *mut FILE) -> off_t {
    unsafe { __ftello(f) }
}
