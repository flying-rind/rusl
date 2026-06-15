//! 对应 musl src/stdio/open_memstream.c
//! 创建动态内存流，自动增长的只写流

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

const BUFSIZ: usize = 1024;

// errno 常量
const EINVAL: c_int = 22;

/// 内部 cookie
struct MsCookie {
    bufp: *mut *mut c_char,
    sizep: *mut usize,
    pos: usize,
    buf: *mut u8,
    len: usize,
    space: usize,
}

/// 动态内存流 FILE 包装
#[repr(C)]
struct MsFile {
    file: FILE,
    cookie: MsCookie,
    buf: [u8; BUFSIZ],
}

/// ms_write 回调 — 动态增长写入。
unsafe extern "C" fn ms_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut MsCookie);

        // 先刷新 FILE 写缓冲区中的待写数据
        let len2 = if f.wpos > f.wbase {
            f.wpos as usize - f.wbase as usize
        } else {
            0
        };
        if len2 > 0 {
            f.wpos = f.wbase;
            let _written = ms_write(f, f.wpos, len2);
            if _written < len2 {
                return 0;
            }
        }

        // 若空间不足，扩容
        if len + c.pos >= c.space {
            let new_space = (2 * c.space + 1).max(c.pos + len + 1);
            let newbuf = realloc(c.buf as *mut c_void, new_space);
            if newbuf.is_null() {
                return 0;
            }
            c.buf = newbuf as *mut u8;
            // 清零新分配部分
            let old_space = c.space;
            c.space = new_space;
            if new_space > old_space {
                core::ptr::write_bytes(
                    c.buf.add(old_space),
                    0,
                    new_space - old_space,
                );
            }
            // 更新调用者的指针
            *c.bufp = c.buf as *mut c_char;
        }

        // 写入数据
        core::ptr::copy_nonoverlapping(buf, c.buf.add(c.pos), len);
        c.pos += len;
        if c.pos >= c.len {
            c.len = c.pos;
        }
        // 实时更新调用者可见的大小
        *c.sizep = c.pos;

        len
    }
}

/// ms_seek 回调
unsafe extern "C" fn ms_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut MsCookie);

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

        // SSIZE_MAX check (simplified)
        let ssize_max = i64::MAX;
        if off < -base || off > ssize_max - base {
            *__errno_location() = EINVAL;
            return -1;
        }

        c.pos = (base + off) as usize;
        c.pos as i64
    }
}

/// ms_close 回调
unsafe extern "C" fn ms_close(f: *mut FILE) -> c_int {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut MsCookie);
        // 确保以 '\0' 结尾
        *c.buf.add(c.len) = 0;
        // 更新调用者输出参数
        *c.bufp = c.buf as *mut c_char;
        *c.sizep = c.len;
        // 注意：缓冲区不释放，所有权归调用者
        0
    }
}

/// 创建动态内存流。
/// - bufp: 输出参数，关闭时写入最终缓冲区地址
/// - sizep: 输出参数，实时更新缓冲区大小（不含 NULL 终止符）
/// 返回新创建的只写 FILE 指针，失败返回 NULL
///
/// [Visibility]: User — <stdio.h> POSIX.1-2008 标准函数。
#[no_mangle]
pub extern "C" fn open_memstream(
    bufp: *mut *mut c_char,
    sizep: *mut usize,
) -> *mut FILE {
    unsafe {
        // 分配 MsFile
        let ptr = malloc(core::mem::size_of::<MsFile>());
        if ptr.is_null() {
            return core::ptr::null_mut();
        }

        // 分配初始缓冲区 (1 字节)
        let init_buf = malloc(1);
        if init_buf.is_null() {
            free(ptr);
            return core::ptr::null_mut();
        }

        let ms_file = &mut *(ptr as *mut MsFile);

        // 清零 FILE 和 cookie
        core::ptr::write_bytes(
            ms_file as *mut MsFile as *mut u8,
            0,
            core::mem::size_of::<MsFile>(),
        );

        let f = &mut ms_file.file;
        let c = &mut ms_file.cookie;

        c.bufp = bufp;
        c.sizep = sizep;
        c.pos = 0;
        c.len = 0;
        c.space = 1;
        c.buf = init_buf as *mut u8;
        *(init_buf as *mut u8).add(0) = 0;

        *sizep = 0;
        *bufp = init_buf as *mut c_char;

        f.flags = F_NORD;
        f.fd = -1;
        f.buf = ms_file.buf.as_mut_ptr();
        f.buf_size = BUFSIZ;
        f.lbf = EOF;
        f.write = Some(ms_write);
        f.seek = Some(ms_seek);
        f.close = Some(ms_close);
        f.mode = -1;

        f.lock = -1;
        f.cookie = c as *mut MsCookie as *mut c_void;

        super::ofl_add::__ofl_add(f)
    }
}
