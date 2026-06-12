//! 对应 musl src/stdio/__stdout_write.c
//! stdout 专用写函数 —— 首次写入时完成延迟初始化

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// stdout 延迟初始化写函数，首次调用后替换为 __stdio_write
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdout_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let f_ref = &mut *f;
    // 首次调用：替换为 __stdio_write
    f_ref.write = Some(super::__stdio_write::__stdio_write);

    // 简化：设为行缓冲模式
    if f_ref.flags & F_SVB == 0 {
        f_ref.lbf = b'\n' as i32;
    }

    super::__stdio_write::__stdio_write(f, buf, len)
}
