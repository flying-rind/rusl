//! lsearch/lfind — 无序数组线性搜索与自动追加（惰性去重集合）。
//! 对 C ABI 导出符号：`lsearch`, `lfind`。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use super::types::CmpFn;

/// 线性搜索并在未找到时追加到数组末尾。
///
/// 遍历 `base[0..*nelp]`，使用 `compar` 比较每个元素与 `key`。
/// - 找到匹配元素：返回该元素的指针。
/// - 未找到：将 `key` 指向的数据复制到 `base[*nelp * width]`，递增 `*nelp`，返回新元素的指针。
///
/// # Safety
///
/// 调用者必须确保：
/// - `key` 指向至少 `width` 字节的有效可读内存。
/// - `base` 指向至少 `(*nelp + 1) * width` 字节的有效可写内存（为可能的追加预留空间）。
/// - `nelp` 为有效非空指针。
/// - `compar` 为有效的比较函数，接收两个 `*const c_void` 返回负数/零/正数。
#[no_mangle]
pub unsafe extern "C" fn lsearch(
    key: *const c_void,
    base: *mut c_void,
    nelp: *mut usize,
    width: usize,
    compar: Option<CmpFn>,
) -> *mut c_void {
    let n = *nelp;
    let p = base as *const u8;
    let cmp = match compar {
        Some(c) => c,
        None => return core::ptr::null_mut(),
    };
    for i in 0..n {
        let elem = p.add(i * width) as *const c_void;
        if cmp(key, elem) == 0 {
            return elem as *mut c_void;
        }
    }
    // 未找到：将 key 的数据复制到数组末尾
    let dst = (base as *mut u8).add(n * width);
    core::ptr::copy_nonoverlapping(key as *const u8, dst, width);
    *nelp = n + 1;
    dst as *mut c_void
}

/// 线性搜索（只读版本）。
///
/// 遍历 `base[0..*nelp]`，使用 `compar` 比较每个元素与 `key`。
/// - 找到匹配元素：返回该元素的指针。
/// - 未找到：返回 null（不修改数组）。
///
/// # Safety
///
/// 调用者必须确保：
/// - `key` 指向至少 `width` 字节的有效可读内存。
/// - `base` 指向至少 `*nelp * width` 字节的有效可读内存。
/// - `nelp` 为有效非空指针。
/// - `compar` 为有效的比较函数。
#[no_mangle]
pub unsafe extern "C" fn lfind(
    key: *const c_void,
    base: *const c_void,
    nelp: *mut usize,
    width: usize,
    compar: Option<CmpFn>,
) -> *mut c_void {
    let n = *nelp;
    let p = base as *const u8;
    let cmp = match compar {
        Some(c) => c,
        None => return core::ptr::null_mut(),
    };
    for i in 0..n {
        let elem = p.add(i * width) as *const c_void;
        if cmp(key, elem) == 0 {
            return elem as *mut c_void;
        }
    }
    core::ptr::null_mut()
}
