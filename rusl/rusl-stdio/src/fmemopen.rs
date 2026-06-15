//! fmemopen — 创建内存流 FILE 对象。
//! 对应 musl src/stdio/fmemopen.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn __errno_location() -> *mut c_int;
}

const UNGET: usize = 8;
const BUFSIZ: usize = 1024;

// errno 常量
const EINVAL: c_int = 22;
const ENOMEM: c_int = 12;

/// 内存流内部 cookie
struct MemCookie {
    pos: usize,
    len: usize,
    size: usize,
    buf: *mut u8,
    mode: u8,
}

/// 内存流 FILE 包装
#[repr(C)]
struct MemFile {
    file: FILE,
    cookie: MemCookie,
    buf: [u8; UNGET + BUFSIZ],
    internal_buf: *mut u8,
    internal_buf_size: usize,
}

/// mem_write 回调 — 向内存流写入。
unsafe extern "C" fn mem_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut MemCookie);

        // 先刷新 FILE 缓冲区中的待写数据
        let len2 = if f.wpos > f.wbase {
            f.wpos as usize - f.wbase as usize
        } else {
            0
        };
        if len2 > 0 {
            f.wpos = f.wbase;
            // 递归写入待写数据
            let written = mem_write(f, f.wpos, len2);
            if written < len2 {
                return 0;
            }
        }

        // 追加模式：从末尾开始
        if c.mode == b'a' {
            c.pos = c.len;
        }

        let rem = c.size - c.pos;
        let mut write_len = len;
        if write_len > rem {
            write_len = rem;
        }

        if write_len > 0 {
            core::ptr::copy_nonoverlapping(buf, c.buf.add(c.pos), write_len);
            c.pos += write_len;
        }

        if c.pos > c.len {
            c.len = c.pos;
            if c.len < c.size {
                *c.buf.add(c.len) = 0;
            } else if (f.flags & F_NORD) != 0 && c.size > 0 {
                *c.buf.add(c.size - 1) = 0;
            }
        }

        write_len
    }
}

/// mem_read 回调 — 从内存流读取。
unsafe extern "C" fn mem_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut MemCookie);

        let rem = c.len.saturating_sub(c.pos);
        if c.pos > c.len {
            // 不应该出现，但安全处理
            return 0;
        }
        let mut read_len = len;
        if read_len > rem {
            read_len = rem;
            f.flags |= F_EOF;
        }

        if read_len > 0 {
            core::ptr::copy_nonoverlapping(c.buf.add(c.pos), buf, read_len);
            c.pos += read_len;
        }

        // 预读到 FILE 内部缓冲区
        let rem_after = c.len.saturating_sub(c.pos);
        if rem_after > 0 {
            let preload = core::cmp::min(rem_after, f.buf_size);
            core::ptr::copy_nonoverlapping(c.buf.add(c.pos), f.buf, preload);
            f.rpos = f.buf;
            f.rend = f.buf.add(preload);
            c.pos += preload;
        }

        read_len
    }
}

/// mem_seek 回调 — 在内存流中移动位置。
unsafe extern "C" fn mem_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut MemCookie);

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

        if off < -base || off > (c.size as i64) - base {
            *__errno_location() = EINVAL;
            return -1;
        }

        c.pos = (base + off) as usize;
        c.pos as i64
    }
}

/// mem_close 回调 — 关闭内存流。
unsafe extern "C" fn mem_close(_f: *mut FILE) -> c_int {
    0
}

/// fmemopen — 创建对内存缓冲区进行 I/O 的 FILE 流。
///
/// [Visibility]: User — <stdio.h> POSIX.1-2008 标准函数。
#[no_mangle]
pub extern "C" fn fmemopen(
    buf: *mut c_void,
    size: usize,
    mode: *const c_char,
) -> *mut FILE {
    unsafe {
        // 校验 mode 首字符
        let first = *mode as u8;
        if first != b'r' && first != b'w' && first != b'a' {
            *__errno_location() = EINVAL;
            return core::ptr::null_mut();
        }

        // 检查是否含 '+'
        let mut plus = false;
        {
            let mut i = 0;
            loop {
                let ch = *mode.add(i);
                if ch == 0 { break; }
                if ch as u8 == b'+' { plus = true; break; }
                i += 1;
            }
        }

        // 若 buf 为 NULL 且 size 过大
        if buf.is_null() && size > isize::MAX as usize {
            *__errno_location() = ENOMEM;
            return core::ptr::null_mut();
        }

        // 分配 MemFile
        let ptr = malloc(core::mem::size_of::<MemFile>());
        if ptr.is_null() {
            return core::ptr::null_mut();
        }

        let mem_file = &mut *(ptr as *mut MemFile);

        // 清零（不包含 buf 数组和 internal_buf，会在后面单独初始化）
        let zero_len = &mem_file.internal_buf as *const _ as usize
            - mem_file as *const _ as usize;
        core::ptr::write_bytes(mem_file as *mut MemFile as *mut u8, 0, zero_len);

        mem_file.internal_buf = core::ptr::null_mut();
        mem_file.internal_buf_size = 0;

        let f = &mut mem_file.file;
        let c = &mut mem_file.cookie;

        // 设置 FILE 字段
        f.cookie = c as *mut MemCookie as *mut c_void;
        f.fd = -1;
        f.lbf = EOF;
        f.buf = mem_file.buf.as_mut_ptr().add(UNGET);
        f.buf_size = BUFSIZ;

        // 若 buf 为 NULL，内部分配
        let actual_buf: *mut u8;
        if buf.is_null() {
            let internal = malloc(size);
            if internal.is_null() {
                free(ptr);
                return core::ptr::null_mut();
            }
            core::ptr::write_bytes(internal as *mut u8, 0, size);
            mem_file.internal_buf = internal as *mut u8;
            mem_file.internal_buf_size = size;
            actual_buf = internal as *mut u8;
        } else {
            actual_buf = buf as *mut u8;
        }

        // 初始化 cookie
        c.buf = actual_buf;
        c.size = size;
        c.mode = first;

        // 设置文件标志和初始状态
        if !plus {
            f.flags = if first == b'r' { F_NOWR } else { F_NORD };
        }
        if first == b'r' {
            c.len = size;
        } else if first == b'a' {
            let n = crate::import::strnlen(actual_buf as *const c_char, size);
            c.len = n;
            c.pos = n;
            if plus && n < size {
                *c.buf.add(n) = 0;
            }
        } else if plus && first == b'w' {
            *c.buf = 0;
        }

        // 设置操作函数指针
        f.read = Some(mem_read);
        f.write = Some(mem_write);
        f.seek = Some(mem_seek);
        f.close = Some(mem_close);

        // 单线程：无需锁
        f.lock = -1;

        // 注册到全局打开文件链表
        super::ofl_add::__ofl_add(f)
    }
}
