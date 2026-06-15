//! fopencookie — 创建用户自定义回调驱动的 FILE 流。
//! 对应 musl src/stdio/fopencookie.c
//! GNU 扩展接口（需 _GNU_SOURCE）。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int, c_void};
use super::stdio_impl::{FILE, F_NOWR, F_NORD, F_EOF, F_ERR, EOF};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn __errno_location() -> *mut c_int;
}

// errno 常量
const EINVAL: c_int = 22;
const ENOTSUP: c_int = 95;

const UNGET: usize = 8;
const BUFSIZ: usize = 1024;

/// cookie_io_functions_t: 用户提供的 I/O 回调函数集合。
#[repr(C)]
pub struct cookie_io_functions_t {
    pub read: Option<unsafe extern "C" fn(*mut c_void, *mut c_char, usize) -> isize>,
    pub write: Option<unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> isize>,
    pub seek: Option<unsafe extern "C" fn(*mut c_void, *mut i64, c_int) -> c_int>,
    pub close: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
}

/// 内部 cookie 状态
struct FCookie {
    cookie: *mut c_void,
    iofuncs: cookie_io_functions_t,
}

/// 自定义回调流 FILE 包装
#[repr(C)]
struct CookieFile {
    file: FILE,
    fc: FCookie,
    buf: [u8; UNGET + BUFSIZ],
}

/// cookie_read 回调封装
unsafe extern "C" fn cookieread(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    unsafe {
        let f = &mut *f;
        let fc = &*(f.cookie as *const FCookie);

        if fc.iofuncs.read.is_none() {
            f.flags |= F_EOF;
            return 0;
        }

        let read_fn = fc.iofuncs.read.unwrap();
        let len2 = if f.buf_size > 0 {
            len.saturating_sub(1)
        } else {
            len
        };

        let mut readlen: usize = 0;

        if len2 > 0 {
            let ret = read_fn(fc.cookie, buf as *mut c_char, len2) as isize;
            if ret <= 0 {
                f.flags |= if ret == 0 { F_EOF } else { F_ERR };
                f.rpos = f.buf;
                f.rend = f.buf;
                return readlen;
            }
            readlen = ret as usize;
        }

        // 若还有剩余 1 字节需要读（预读到内部缓冲区）
        if f.buf_size == 0 || len.saturating_sub(readlen) <= 1 {
            return readlen;
        }

        // 预读 1 字节
        f.rpos = f.buf;
        let ret = read_fn(fc.cookie, f.rpos as *mut c_char, 1) as isize;
        if ret <= 0 {
            f.flags |= if ret == 0 { F_EOF } else { F_ERR };
            f.rend = f.buf;
            return readlen;
        }
        f.rend = f.rpos.add(1);
        *(buf.add(readlen)) = *f.rpos;
        f.rpos = f.rpos.add(1);

        readlen + 1
    }
}

/// cookie_write 回调封装
unsafe extern "C" fn cookiewrite(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    unsafe {
        let f = &mut *f;
        let fc = &*(f.cookie as *const FCookie);

        if fc.iofuncs.write.is_none() {
            return len; // 静默丢弃
        }

        let write_fn = fc.iofuncs.write.unwrap();

        // 先刷新 FILE 写缓冲区中的待写数据
        let len2 = if f.wpos > f.wbase {
            f.wpos as usize - f.wbase as usize
        } else {
            0
        };
        if len2 > 0 {
            f.wpos = f.wbase;
            let written = cookiewrite(f, f.wpos, len2);
            if written < len2 {
                return 0;
            }
        }

        let ret = write_fn(fc.cookie, buf as *const c_char, len) as isize;
        if ret < 0 {
            f.wpos = f.buf;
            f.wbase = f.buf;
            f.wend = f.buf;
            f.flags |= F_ERR;
            return 0;
        }
        ret as usize
    }
}

/// cookie_seek 回调封装
unsafe extern "C" fn cookieseek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    unsafe {
        let f = &mut *f;
        let fc = &*(f.cookie as *const FCookie);

        if (whence as u32) > 2 {
            *__errno_location() = EINVAL;
            return -1;
        }

        if fc.iofuncs.seek.is_none() {
            *__errno_location() = ENOTSUP;
            return -1;
        }

        let seek_fn = fc.iofuncs.seek.unwrap();
        let mut off_copy = off;
        let res = seek_fn(fc.cookie, &mut off_copy, whence);
        if res < 0 {
            return -1;
        }
        off_copy
    }
}

/// cookie_close 回调封装
unsafe extern "C" fn cookieclose(f: *mut FILE) -> c_int {
    unsafe {
        let f = &mut *f;
        let fc = &*(f.cookie as *const FCookie);
        if let Some(close_fn) = fc.iofuncs.close {
            close_fn(fc.cookie)
        } else {
            0
        }
    }
}

/// 创建用户自定义回调流。
///
/// [Visibility]: User — <stdio.h> GNU 扩展函数。
#[no_mangle]
pub extern "C" fn fopencookie(
    cookie: *mut c_void,
    mode: *const c_char,
    iofuncs: cookie_io_functions_t,
) -> *mut FILE {
    unsafe {
        // 校验 mode 首字符
        let first = *mode as u8;
        if first != b'r' && first != b'w' && first != b'a' {
            *__errno_location() = EINVAL;
            return core::ptr::null_mut();
        }

        // 分配 CookieFile
        let ptr = malloc(core::mem::size_of::<CookieFile>());
        if ptr.is_null() {
            return core::ptr::null_mut();
        }

        let cookie_file = &mut *(ptr as *mut CookieFile);

        // 清零 FILE 和 fc（不含 buf 数组）
        let zero_len = &cookie_file.buf as *const _ as usize
            - cookie_file as *const _ as usize;
        core::ptr::write_bytes(cookie_file as *mut CookieFile as *mut u8, 0, zero_len);

        let f = &mut cookie_file.file;
        let fc = &mut cookie_file.fc;

        // 施加模式限制
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
        if !plus {
            f.flags = if first == b'r' { F_NOWR } else { F_NORD };
        }

        // 设置 fc
        fc.cookie = cookie;
        fc.iofuncs = iofuncs;

        f.fd = -1;
        f.cookie = fc as *mut FCookie as *mut c_void;
        f.buf = cookie_file.buf.as_mut_ptr().add(UNGET);
        f.buf_size = BUFSIZ;
        f.lbf = EOF;

        f.read = Some(cookieread);
        f.write = Some(cookiewrite);
        f.seek = Some(cookieseek);
        f.close = Some(cookieclose);

        f.lock = -1;

        super::ofl_add::__ofl_add(f)
    }
}
