//! stpncpy — 将 s 中最多 n 个字符复制到 d。若 s 长度小于 n，剩余用 '\0' 填充。返回写入的最后一个非 null 字符之后的位置。

#![allow(unused_imports, unused_variables)]

/// stpncpy — 将 s 中最多 n 个字符复制到 d。若 s 长度小于 n，剩余用 '\0' 填充。返回 d + min(strlen(s), n)。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 n 字节
/// - s 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn __stpncpy(
    d: *mut core::ffi::c_char,
    s: *const core::ffi::c_char,
    n: usize,
) -> *mut core::ffi::c_char {
    let d8 = d as *mut u8;
    let s8 = s as *const u8;
    let mut i = 0;
    while i < n {
        let byte = unsafe { *s8.add(i) };
        unsafe { *d8.add(i) = byte };
        if byte == 0 {
            let null_pos = i;
            i += 1;
            while i < n {
                unsafe { *d8.add(i) = 0 };
                i += 1;
            }
            return d.add(null_pos);
        }
        i += 1;
    }
    d.add(n)
}

#[no_mangle]
pub unsafe extern "C" fn stpncpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char {
    unsafe { __stpncpy(d, s, n) }
}

/// 安全的 Rust 内部实现。
pub(crate) fn stpncpy_impl(dst: &mut [u8], src: &core::ffi::CStr, n: usize) -> *mut u8 {
    let src_bytes = src.to_bytes();
    let copy_len = n.min(src_bytes.len());
    dst[..copy_len].copy_from_slice(&src_bytes[..copy_len]);
    if copy_len < n {
        for i in copy_len..n {
            dst[i] = 0;
        }
        unsafe { dst.as_mut_ptr().add(copy_len) }
    } else {
        unsafe { dst.as_mut_ptr().add(n) }
    }
}
