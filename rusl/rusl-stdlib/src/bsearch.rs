//! bsearch —— 二分查找。对外导出 C ABI 兼容的 `bsearch` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;

/// 在已排序数组中执行二分查找。
///
/// # Safety
///
/// - `base` 必须指向按 `cmp` 升序排列的数组，包含 `nel` 个元素，每个元素宽度为 `width` 字节。
/// - `cmp` 必须是比较函数，返回 <0、=0 或 >0。比较函数不得修改数组元素。
/// - `key` 必须是对比关键字的有效指针。
/// - `nel` 为 0 时 `base` 可为空（或任意值）。
///
/// # 返回值
///
/// - 找到匹配元素：返回指向该元素的 `*mut c_void` 指针。
/// - 未找到匹配元素：返回 `null`。
#[no_mangle]
pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    nel: usize,
    width: usize,
    cmp: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void {
    let cmp = match cmp {
        Some(f) => f,
        None => return core::ptr::null_mut(),
    };
    let mut base = base as *const u8;
    let mut nel = nel;
    while nel > 0 {
        let try_ptr = unsafe { base.add(width * (nel / 2)) };
        let sign = cmp(key, try_ptr as *const c_void);
        if sign < 0 {
            nel /= 2;
        } else if sign > 0 {
            base = unsafe { try_ptr.add(width) };
            nel -= nel / 2 + 1;
        } else {
            return try_ptr as *mut c_void;
        }
    }
    core::ptr::null_mut()
}
