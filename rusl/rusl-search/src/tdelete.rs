//! tdelete — AVL 二叉树节点删除。
//! 对 C ABI 导出符号：`tdelete`。

#![allow(unused_imports, unused_variables)]

#[cfg(test)]
use alloc::boxed::Box;
#[cfg(test)]
use alloc::vec::Vec;

use core::ffi::c_void;
use core::ptr;
use super::types::Node;

/// AVL 树高度安全上界，与 tsearch.rs 保持一致。
const MAXH: usize = core::mem::size_of::<*mut c_void>() * 8 * 3 / 2;

extern "C" {
    fn free(ptr: *mut c_void);
}

/// 从二叉树中删除与 `key` 匹配的节点（内部实现）。
///
/// 删除后执行 AVL 再平衡操作。
/// - 找到并删除：释放节点内存，返回被删除节点的父节点指针。
///   （若删除根节点，返回指向新根的指针的指针。）
/// - 未找到：返回 null。
///
/// # Safety
///
/// 调用者必须确保：
/// - `rootp` 为有效指针，指向非空树的根指针。
/// - `compar` 为有效的比较函数。
/// - 删除后不再通过已删除节点的指针访问数据。
pub(crate) unsafe fn tdelete_impl(
    key: *const c_void,
    rootp: *mut *mut c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void {
    if rootp.is_null() {
        return ptr::null_mut();
    }

    let cmp = match compar {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    // 路径栈（多一个位置用于 sentinel，与 C 源码一致）
    let mut a: [*mut *mut c_void; MAXH + 1] = [ptr::null_mut(); MAXH + 1];
    let mut n: *mut Node = (*rootp) as *mut Node;
    let mut i: usize = 0;

    // a[0] 和 a[1] 都指向 rootp（a[1] 作为根被删除时的父指针 sentinel）
    a[i] = rootp;
    i += 1;
    a[i] = rootp;
    i += 1;

    // 沿树查找待删除节点
    loop {
        if n.is_null() {
            return ptr::null_mut();
        }
        let c = cmp(key, (*n).key);
        if c == 0 {
            break;
        }
        let dir = (c > 0) as usize;
        a[i] = &mut (*n).a[dir] as *mut *mut c_void;
        i += 1;
        n = (*n).a[dir] as *mut Node;
    }

    // parent 是待删除节点的父节点指针（用于返回）
    let parent = *a[i - 2];

    let child: *mut c_void;
    if !(*n).a[0].is_null() {
        // 待删除节点有左子树：找到其中序前驱（左子树的最右节点）
        // 将被删节点的 key 与前驱交换，然后删除前驱节点
        let deleted = n;
        a[i] = &mut (*n).a[0] as *mut *mut c_void;
        i += 1;
        n = (*n).a[0] as *mut Node;
        // 一直向右
        while !(*n).a[1].is_null() {
            a[i] = &mut (*n).a[1] as *mut *mut c_void;
            i += 1;
            n = (*n).a[1] as *mut Node;
        }
        // 交换 key（前驱节点替代被删节点的位置）
        (*deleted).key = (*n).key;
        child = (*n).a[0];
    } else {
        // 待删除节点没有左子树：直接用右子树替代
        child = (*n).a[1];
    }

    // 释放节点内存
    free(n as *mut c_void);

    // 用子节点替代已释放的节点
    i -= 1;
    *a[i] = child;

    // 从父节点开始向上再平衡
    // C 源码：while (--i && __tsearch_balance(a[i]));
    // 先递减 i，若 i 为 0 则停止，否则调用 __tsearch_balance
    loop {
        if i == 0 {
            break;
        }
        i -= 1;
        if i == 0 {
            break;
        }
        if super::tsearch::__tsearch_balance(a[i]) == 0 {
            break;
        }
    }

    parent
}

/// Debug helper to test tdelete_impl with known inputs
#[cfg(test)]
pub(crate) unsafe fn test_tdelete_leaf_debug() -> *mut c_void {
    use core::ptr;
    use super::types::Node;
    extern "C" { fn malloc(size: usize) -> *mut c_void; fn free(ptr: *mut c_void); }

    let keys = [10i32, 20, 30];
    let left = malloc(core::mem::size_of::<Node>()) as *mut Node;
    (*left).key = &keys[0] as *const i32 as *const c_void;
    (*left).a = [ptr::null_mut(), ptr::null_mut()];
    (*left).h = 1;

    let right = malloc(core::mem::size_of::<Node>()) as *mut Node;
    (*right).key = &keys[2] as *const i32 as *const c_void;
    (*right).a = [ptr::null_mut(), ptr::null_mut()];
    (*right).h = 1;

    let root = malloc(core::mem::size_of::<Node>()) as *mut Node;
    (*root).key = &keys[1] as *const i32 as *const c_void;
    (*root).a = [left as *mut c_void, right as *mut c_void];
    (*root).h = 2;

    let rootp = malloc(core::mem::size_of::<*mut c_void>()) as *mut *mut c_void;
    *rootp = root as *mut c_void;

    let key: i32 = 10;
    let result = tdelete_impl(
        &key as *const i32 as *const c_void,
        rootp,
        Some(cmp_int_debug),
    );

    // Cleanup: free rootp Box (root node cleanup is handled by tdelete_impl or leaked test tree)
    free(rootp as *mut c_void);

    result
}

#[cfg(test)]
unsafe extern "C" fn cmp_int_debug(a: *const c_void, b: *const c_void) -> i32 {
    let va = *(a as *const i32);
    let vb = *(b as *const i32);
    if va < vb { -1 } else if va > vb { 1 } else { 0 }
}

/// Test the tdelete_impl directly from a standalone helper
#[cfg(test)]
mod debug_tests {
    use rusl_core::test;

    test!("test_tdelete_debug_helper" {
        unsafe {
            let result = super::test_tdelete_leaf_debug();
            assert!(!result.is_null(), "tdelete_impl returned null!");
        }
    });
}

/// 从二叉树中删除与 `key` 匹配的节点（C ABI 导出符号）。
///
/// 内部委托给 `tdelete_impl`。测试模式下不生成此符号，
/// 测试通过 `tdelete_impl` 直接调用，避免与 libc 的 `tdelete` 符号冲突。
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn tdelete(
    key: *const c_void,
    rootp: *mut *mut c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void {
    unsafe { tdelete_impl(key, rootp, compar) }
}

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use alloc::string::String;
    use super::*;
    use core::ptr;

    /// 比较 i32 值的辅助函数。
    unsafe extern "C" fn cmp_int(a: *const c_void, b: *const c_void) -> i32 {
        let va = *(a as *const i32);
        let vb = *(b as *const i32);
        if va < vb { -1 } else if va > vb { 1 } else { 0 }
    }

    /// 手动构建一个小型 AVL 树用于删除测试。
    /// 返回 (rootp, keys_owned) 其中 keys_owned（Box 堆分配）防止 key 被提前释放。
    /// 所有节点通过 C `malloc` 分配（与 tsearch_impl 一致），由 tdelete_impl 内部 `free` 释放。
    unsafe fn build_test_tree() -> (*mut *mut c_void, Box<[i32; 3]>) {
        extern "C" { fn malloc(size: usize) -> *mut c_void; }
        let keys = Box::new([10i32, 20, 30]);
        // 构建树：
        //        20
        //       /  \
        //     10    30
        let left = malloc(core::mem::size_of::<Node>()) as *mut Node;
        (*left).key = &keys[0] as *const i32 as *const c_void;
        (*left).a = [ptr::null_mut(), ptr::null_mut()];
        (*left).h = 1;

        let right = malloc(core::mem::size_of::<Node>()) as *mut Node;
        (*right).key = &keys[2] as *const i32 as *const c_void;
        (*right).a = [ptr::null_mut(), ptr::null_mut()];
        (*right).h = 1;

        let root = malloc(core::mem::size_of::<Node>()) as *mut Node;
        (*root).key = &keys[1] as *const i32 as *const c_void;
        (*root).a = [left as *mut c_void, right as *mut c_void];
        (*root).h = 2;

        let rootp = malloc(core::mem::size_of::<*mut c_void>()) as *mut *mut c_void;
        *rootp = root as *mut c_void;
        (rootp, keys)
    }

    /// 清理树中未被 tdelete_impl 释放的剩余节点（通过 C `free`）。
    unsafe fn cleanup_test_tree(rootp: *mut *mut c_void) {
        extern "C" { fn free(ptr: *mut c_void); }
        if rootp.is_null() {
            return;
        }
        let root = *rootp;
        if !root.is_null() {
            let node = &*(root as *const Node);
            if !node.a[0].is_null() {
                free(node.a[0]);
            }
            if !node.a[1].is_null() {
                free(node.a[1]);
            }
            free(root);
        }
        free(rootp as *mut c_void);
    }

    test!("test_tdelete_leaf" {
        // 测试删除叶节点（10）。
        unsafe {
            let (rootp, _keys) = build_test_tree();
            let key: i32 = 10;
            let result = tdelete_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            // 删除成功返回非空指针
            assert!(!result.is_null());
            cleanup_test_tree(rootp);
        }
    });

    test!("test_tdelete_root" {
        // 测试删除根节点（20）。
        unsafe {
            let (rootp, _keys) = build_test_tree();
            let key: i32 = 20;
            let result = tdelete_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            // 根删除应返回指针（指向原根）
            assert!(!result.is_null());
            cleanup_test_tree(rootp);
        }
    });

    test!("test_tdelete_not_found" {
        // 测试删除不存在的 key（应返回 null）。
        unsafe {
            let (rootp, _keys) = build_test_tree();
            let key: i32 = 99;
            let result = tdelete_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            assert_eq!(result, ptr::null_mut());
            cleanup_test_tree(rootp);
        }
    });

    test!("test_tdelete_empty_tree" {
        // 测试在空树上删除（*rootp = null）。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let key: i32 = 1;
            let result = tdelete_impl(
                &key as *const i32 as *const c_void,
                &mut root as *mut *mut c_void,
                Some(cmp_int),
            );
            assert_eq!(result, ptr::null_mut());
        }
    });

    test!("test_tdelete_null_rootp" {
        // 测试 null rootp。
        unsafe {
            let key: i32 = 1;
            let result = tdelete_impl(
                &key as *const i32 as *const c_void,
                ptr::null_mut(),
                Some(cmp_int),
            );
            assert!(result.is_null());
        }
    });

    test!("test_tdelete_then_find" {
        // 测试删除后再次查找被删元素（应找不到）。
        unsafe {
            let (rootp, _keys) = build_test_tree();
            let key: i32 = 10;
            tdelete_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            // 用 tfind 验证已删除
            let root_const: *const c_void = *rootp as *const c_void;
            let mut rootp_const: *const c_void = root_const;
            let found = super::super::tfind::tfind_impl(
                &key as *const i32 as *const c_void,
                &mut rootp_const as *mut *const c_void,
                Some(cmp_int),
            );
            assert_eq!(found, ptr::null_mut());
            cleanup_test_tree(rootp);
        }
    });

    test!("test_tdelete_all" {
        // 测试连续删除所有节点。
        unsafe {
            extern "C" { fn free(ptr: *mut c_void); }
            let (rootp, _keys) = build_test_tree();
            // 删除三个节点
            for k in [10i32, 30, 20] {
                tdelete_impl(
                    &k as *const i32 as *const c_void,
                    rootp,
                    Some(cmp_int),
                );
            }
            // 树应为空
            assert_eq!(*rootp, ptr::null_mut());
            free(rootp as *mut c_void);
        }
    });

    test!("test_tdelete_null_compar" {
        // 测试使用 null compar（返回 null 而非崩溃）。
        unsafe {
            let (rootp, _keys) = build_test_tree();
            let key: i32 = 10;
            let result = tdelete_impl(
                &key as *const i32 as *const c_void,
                rootp,
                None,
            );
            assert!(result.is_null());
            cleanup_test_tree(rootp);
        }
    });
}