//! fread — 从 FILE 流中读取指定数量的元素到用户缓冲区。
//! 对应 musl src/stdio/fread.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use super::stdio_impl::FILE;

/// __freadx — 内部读实现（不加锁）。
unsafe fn __freadx(dest: *mut u8, len: usize, f: *mut FILE) -> usize {
    let f_ref = &mut *f;
    let mut total: usize = 0;

    while total < len {
        // 先消费读缓冲区中已有的数据
        if f_ref.rpos != f_ref.rend {
            let avail = (f_ref.rend as usize).wrapping_sub(f_ref.rpos as usize);
            let n = if avail < len - total { avail } else { len - total };
            core::ptr::copy_nonoverlapping(f_ref.rpos, dest.add(total), n);
            f_ref.rpos = f_ref.rpos.add(n);
            total += n;
            continue;
        }

        // 缓冲区空：确保处于读模式（首次调用时 rpos/rend 为 null）
        if f_ref.rpos.is_null() || f_ref.rend.is_null() {
            if super::__toread::__toread(f) != 0 {
                break;
            }
            // __toread 设置 rpos=rend=buf+buf_size，回循环填充缓冲区
            continue;
        }

        // 大块数据直接读取到目标缓冲区
        if len - total >= f_ref.buf_size {
            if let Some(read_fn) = f_ref.read {
                let n = read_fn(f, dest.add(total), len - total);
                if n == 0 {
                    break;
                }
                total += n;
            } else {
                break;
            }
        } else {
            // 小块数据：填充内部缓冲区后再复制
            if let Some(read_fn) = f_ref.read {
                let n = read_fn(f, f_ref.buf, f_ref.buf_size);
                if n == 0 {
                    break;
                }
                f_ref.rpos = f_ref.buf;
                f_ref.rend = f_ref.buf.add(n);
            } else {
                break;
            }
        }
    }

    total
}

/// 从 FILE 流 f 中读取 nmemb 个大小为 size 字节的元素到 destv 缓冲区。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fread(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    let len = size.wrapping_mul(nmemb);
    if len == 0 {
        return 0;
    }
    let k = unsafe { __freadx(destv as *mut u8, len, f) };
    k / size
}

/// 免锁版本（弱别名 -> fread）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fread_unlocked(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    let len = size.wrapping_mul(nmemb);
    if len == 0 {
        return 0;
    }
    let k = unsafe { __freadx(destv as *mut u8, len, f) };
    k / size
}
