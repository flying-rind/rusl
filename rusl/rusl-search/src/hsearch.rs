//! hcreate/hdestroy/hsearch — POSIX 哈希表管理（过时接口）。
//! 对 C ABI 导出符号：`hcreate`, `hdestroy`, `hsearch`。
//!
//! 实现：开放寻址哈希表，2^n 表大小，二次探测解决冲突。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_void};
use core::ptr;
use super::types::{ENTRY, ACTION};

/// 最小哈希表大小。
const MINSIZE: usize = 8;

/// 最大哈希表大小（最小的会溢出的 2 的幂）。
const MAXSIZE: usize = usize::MAX / 2 + 1;

/// 内部哈希表结构。
#[repr(C)]
struct __tab {
    entries: *mut ENTRY,
    mask: usize,
    used: usize,
}

/// hsearch_data — 符合 musl `<search.h>` 的全局哈希表句柄。
#[repr(C)]
struct HsearchData {
    tab: *mut __tab,
    _unused1: u32,
    _unused2: u32,
}

/// 全局哈希表实例（对应 C 源码的 `static struct hsearch_data htab`）。
static mut HTAB: HsearchData = HsearchData {
    tab: ptr::null_mut(),
    _unused1: 0,
    _unused2: 0,
};

extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// 哈希函数：对字符串 key 计算哈希值（与 Java String.hashCode 相同算法）。
fn keyhash(k: *const c_char) -> usize {
    let mut p = k as *const u8;
    let mut h: usize = 0;
    unsafe {
        while *p != 0 {
            h = h.wrapping_mul(31).wrapping_add(*p as usize);
            p = p.add(1);
        }
    }
    h
}

/// 字符串比较（替代 C strcmp，避免依赖外部符号）。
#[inline]
unsafe fn str_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut pa = a as *const u8;
    let mut pb = b as *const u8;
    loop {
        let ca = *pa;
        let cb = *pb;
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        pa = pa.add(1);
        pb = pb.add(1);
    }
}

/// 扩容（或初始化）哈希表。
///
/// 对 C 源码 `resize()`。将表大小调整为 >= nel 的 2 的幂，并重新散列已有条目。
/// 失败时恢复旧状态。
unsafe fn resize(nel: usize) -> bool {
    let tab = &mut *HTAB.tab;
    let oldsize = tab.mask + 1;
    let oldtab = tab.entries;

    // 计算新表大小（2 的幂）
    let mut nel = nel;
    if nel > MAXSIZE {
        nel = MAXSIZE;
    }
    let mut newsize = MINSIZE;
    while newsize < nel {
        newsize <<= 1;
    }

    // 分配新条目数组
    let new_entries = calloc(newsize, core::mem::size_of::<ENTRY>()) as *mut ENTRY;
    if new_entries.is_null() {
        return false;
    }

    tab.entries = new_entries;
    tab.mask = newsize - 1;

    // 首次分配，不需要重新散列
    if oldtab.is_null() {
        return true;
    }

    // 重新散列旧条目到新表
    let mut e = oldtab;
    let end = oldtab.add(oldsize);
    while e < end {
        if !(*e).key.is_null() {
            let hash = keyhash((*e).key);
            let mut i = hash;
            let mut j: usize = 1;
            loop {
                let newe = tab.entries.add(i & tab.mask);
                if (*newe).key.is_null() {
                    ptr::copy_nonoverlapping(e, newe, 1);
                    break;
                }
                i = i.wrapping_add(j);
                j += 1;
            }
        }
        e = e.add(1);
    }

    free(oldtab as *mut c_void);
    true
}

/// 在哈希表中查找 key（二次探测）。
///
/// 返回第一个空位或匹配项的指针。
unsafe fn lookup(key: *mut c_char, hash: usize) -> *mut ENTRY {
    let tab = &*HTAB.tab;
    let mut i = hash;
    let mut j: usize = 1;
    loop {
        let e = tab.entries.add(i & tab.mask);
        if (*e).key.is_null() || str_eq((*e).key, key) {
            return e;
        }
        i = i.wrapping_add(j);
        j += 1;
    }
}

/// 创建哈希表（内部实现，对应 `__hcreate_r`）。
unsafe fn hcreate_impl(nel: usize) -> i32 {
    HTAB.tab = calloc(1, core::mem::size_of::<__tab>()) as *mut __tab;
    if HTAB.tab.is_null() {
        return 0;
    }
    if !resize(nel) {
        free(HTAB.tab as *mut c_void);
        HTAB.tab = ptr::null_mut();
        return 0;
    }
    1
}

/// 销毁哈希表（内部实现，对应 `__hdestroy_r`）。
unsafe fn hdestroy_impl() {
    if !HTAB.tab.is_null() {
        if !(*HTAB.tab).entries.is_null() {
            free((*HTAB.tab).entries as *mut c_void);
        }
        free(HTAB.tab as *mut c_void);
        HTAB.tab = ptr::null_mut();
    }
}

/// 查找或插入条目（内部实现，对应 `__hsearch_r`）。
unsafe fn hsearch_impl(item: ENTRY, action: ACTION, retval: *mut *mut ENTRY) -> i32 {
    let key = item.key;
    let hash = keyhash(key);
    let e = lookup(key, hash);

    // 找到已有条目
    if !(*e).key.is_null() {
        *retval = e;
        return 1;
    }

    // 查找模式，未找到
    if matches!(action, ACTION::FIND) {
        *retval = ptr::null_mut();
        return 0;
    }

    // ENTER 模式：插入新条目
    *e = item;
    (*HTAB.tab).used += 1;

    // 负载因子超过 75% 时扩容（used > mask - mask/4）
    let tab = &*HTAB.tab;
    if tab.used > tab.mask - tab.mask / 4 {
        if !resize(2 * tab.used) {
            // 扩容失败：回滚
            let tab = &mut *HTAB.tab;
            tab.used -= 1;
            (*e).key = ptr::null_mut();
            *retval = ptr::null_mut();
            return 0;
        }
        // 扩容后重新查找（表可能已改变）
        let e = lookup(key, hash);
        *retval = e;
    } else {
        *retval = e;
    }
    1
}

// ---- 公共 C ABI 导出 ----

/// 创建哈希表。
#[no_mangle]
pub extern "C" fn hcreate(nel: usize) -> i32 {
    unsafe { hcreate_impl(nel) }
}

/// 销毁哈希表。
#[no_mangle]
pub extern "C" fn hdestroy() {
    unsafe { hdestroy_impl() }
}

/// 搜索或插入哈希表条目。
#[no_mangle]
pub extern "C" fn hsearch(item: ENTRY, action: ACTION) -> *mut ENTRY {
    let mut e: *mut ENTRY = ptr::null_mut();
    unsafe { hsearch_impl(item, action, &mut e) };
    e
}
