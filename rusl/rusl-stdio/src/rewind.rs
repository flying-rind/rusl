//! 对应 musl src/stdio/rewind.c
//! 文件流回绕 —— 将文件位置重置到起始并清除错误状态

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

const SEEK_SET: i32 = 0;

/// 将文件流位置回绕到起始，清除 F_ERR 标志
#[no_mangle]
pub extern "C" fn rewind(f: *mut FILE) {
    unsafe {
        super::fseek::__fseeko_unlocked(f, 0, SEEK_SET);
        let f_ref = &mut *f;
        f_ref.flags &= !super::stdio_impl::F_ERR;
    }
}
