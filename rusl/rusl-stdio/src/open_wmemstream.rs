//! 对应 musl src/stdio/open_wmemstream.c
//! 创建宽字符动态内存流

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

// errno 常量
const EINVAL: c_int = 22;

/// 内部 cookie
struct WmsCookie {
    bufp: *mut *mut c_int,   // wchar_t**
    sizep: *mut usize,
    pos: usize,
    buf: *mut c_int,          // wchar_t*
    len: usize,
    space: usize,
}

/// 宽字符动态内存流 FILE 包装
#[repr(C)]
struct WmsFile {
    file: FILE,
    cookie: WmsCookie,
    buf: [u8; 1],
}

/// wms_write 回调 — 将 multibyte 输入转换为宽字符写入。
unsafe extern "C" fn wms_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut WmsCookie);

        // 先刷新 FILE 写缓冲区中的待写数据
        let len2 = if f.wpos > f.wbase {
            f.wpos as usize - f.wbase as usize
        } else {
            0
        };
        if len2 > 0 {
            f.wpos = f.wbase;
            let written = wms_write(f, f.wpos, len2);
            if written < len2 {
                return 0;
            }
        }

        // 若空间不足，扩容
        if len + c.pos >= c.space {
            let new_space = (2 * c.space + 1).max(c.pos + len + 1);
            // 检查不会溢出 (SSIZE_MAX/4)
            if new_space > (isize::MAX as usize) / 4 {
                return 0;
            }
            let newbuf = realloc(c.buf as *mut c_void, new_space * 4);
            if newbuf.is_null() {
                return 0;
            }
            c.buf = newbuf as *mut c_int;
            // 清零新分配部分
            if new_space > c.space {
                core::ptr::write_bytes(
                    (c.buf as *mut u8).add(c.space * 4),
                    0,
                    (new_space - c.space) * 4,
                );
            }
            c.space = new_space;
            *c.bufp = c.buf;
        }

        // 简化的 multibyte -> wchar_t 转换（仅支持 ASCII 单字节）
        let mut src_idx: usize = 0;
        let mut dst_idx: usize = 0;
        let max_chars = c.space - c.pos;

        while src_idx < len && dst_idx < max_chars {
            let byte = *buf.add(src_idx);
            // ASCII 单字节：直接映射到 wchar_t
            if byte < 0x80 {
                *c.buf.add(c.pos + dst_idx) = byte as c_int;
                src_idx += 1;
                dst_idx += 1;
            } else if byte < 0xC0 {
                // 非法的续字节，跳过
                src_idx += 1;
            } else if byte < 0xE0 {
                // 2 字节序列
                if src_idx + 1 < len {
                    let b1 = *buf.add(src_idx + 1);
                    let cp = ((byte as u32 & 0x1F) << 6) | (b1 as u32 & 0x3F);
                    *c.buf.add(c.pos + dst_idx) = cp as c_int;
                    src_idx += 2;
                    dst_idx += 1;
                } else {
                    break;
                }
            } else if byte < 0xF0 {
                // 3 字节序列
                if src_idx + 2 < len {
                    let b1 = *buf.add(src_idx + 1);
                    let b2 = *buf.add(src_idx + 2);
                    let cp = ((byte as u32 & 0x0F) << 12)
                        | ((b1 as u32 & 0x3F) << 6)
                        | (b2 as u32 & 0x3F);
                    *c.buf.add(c.pos + dst_idx) = cp as c_int;
                    src_idx += 3;
                    dst_idx += 1;
                } else {
                    break;
                }
            } else {
                // 4 字节序列
                if src_idx + 3 < len {
                    let b1 = *buf.add(src_idx + 1);
                    let b2 = *buf.add(src_idx + 2);
                    let b3 = *buf.add(src_idx + 3);
                    let cp = ((byte as u32 & 0x07) << 18)
                        | ((b1 as u32 & 0x3F) << 12)
                        | ((b2 as u32 & 0x3F) << 6)
                        | (b3 as u32 & 0x3F);
                    *c.buf.add(c.pos + dst_idx) = cp as c_int;
                    src_idx += 4;
                    dst_idx += 1;
                } else {
                    break;
                }
            }
        }

        c.pos += dst_idx;
        if c.pos >= c.len {
            c.len = c.pos;
        }
        *c.sizep = c.pos;

        len // 假装全部字节均已消费
    }
}

/// wms_seek 回调
unsafe extern "C" fn wms_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut WmsCookie);

        extern "C" {
            fn __errno_location() -> *mut c_int;
        }

        if (whence as u32) > 2 {
            *__errno_location() = EINVAL;
            return -1;
        }

        let base = match whence {
            0 => 0i64,
            1 => c.pos as i64,
            2 => c.len as i64,
            _ => unreachable!(),
        };

        // SSIZE_MAX/4 check
        let limit = (isize::MAX as i64) / 4;
        if off < -base || off > limit - base {
            *__errno_location() = EINVAL;
            return -1;
        }

        c.pos = (base + off) as usize;
        c.pos as i64
    }
}

/// wms_close 回调
unsafe extern "C" fn wms_close(f: *mut FILE) -> c_int {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut WmsCookie);
        // 确保以 L'\0' 结尾
        *c.buf.add(c.len) = 0;
        *c.bufp = c.buf;
        *c.sizep = c.len;
        0
    }
}

/// 创建宽字符动态内存流。
///
/// [Visibility]: User — <stdio.h> POSIX.1-2008 标准函数。
#[no_mangle]
pub extern "C" fn open_wmemstream(
    bufp: *mut *mut c_int,
    sizep: *mut usize,
) -> *mut FILE {
    unsafe {
        // 分配 WmsFile
        let ptr = malloc(core::mem::size_of::<WmsFile>());
        if ptr.is_null() {
            return core::ptr::null_mut();
        }

        // 分配初始缓冲区（1 个 wchar_t = 4 字节）
        let init_buf = malloc(4);
        if init_buf.is_null() {
            free(ptr);
            return core::ptr::null_mut();
        }

        let wms_file = &mut *(ptr as *mut WmsFile);

        // 清零
        core::ptr::write_bytes(
            wms_file as *mut WmsFile as *mut u8,
            0,
            core::mem::size_of::<WmsFile>(),
        );

        let f = &mut wms_file.file;
        let c = &mut wms_file.cookie;

        c.bufp = bufp;
        c.sizep = sizep;
        c.pos = 0;
        c.len = 0;
        c.space = 1;
        c.buf = init_buf as *mut c_int;
        *c.buf = 0;

        *sizep = 0;
        *bufp = c.buf;

        f.flags = F_NORD;
        f.fd = -1;
        f.buf = wms_file.buf.as_mut_ptr();
        f.buf_size = 0;
        f.lbf = EOF;
        f.write = Some(wms_write);
        f.seek = Some(wms_seek);
        f.close = Some(wms_close);

        f.lock = -1;
        f.cookie = c as *mut WmsCookie as *mut c_void;

        // 设置宽字符方向
        super::fwide::fwide(f, 1);

        super::ofl_add::__ofl_add(f)
    }
}
