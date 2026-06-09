//! tfind — AVL 二叉树只读查找。
//! 对 C ABI 导出符号：`tfind`。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use super::types::Node;

/// 在二叉树中搜索 `key`（内部实现）。
///
/// 沿 AVL 树二分查找与 `key` 匹配的节点。
/// - 找到：返回匹配节点的指针。
/// - 未找到或树为空：返回 null（不修改树结构）。
///
/// # Safety
///
/// 调用者必须确保：
/// - `rootp` 为有效指针（可为指向 null 的指针，表示空树）。
/// - `compar` 为有效的比较函数。
/// - 树的结构在查找期间不被修改。
pub(crate) unsafe fn tfind_impl(
    key: *const c_void,
    rootp: *mut *const c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void {
    if rootp.is_null() {
        return core::ptr::null_mut();
    }
    let cmp = match compar {
        Some(c) => c,
        None => return core::ptr::null_mut(),
    };
    let mut n: *mut Node = (*rootp) as *mut Node;
    loop {
        if n.is_null() {
            break;
        }
        let c = cmp(key, (*n).key);
        if c == 0 {
            break;
        }
        n = (*n).a[(c > 0) as usize] as *mut Node;
    }
    n as *mut c_void
}

/// 在二叉树中搜索 `key`（C ABI 导出符号）。
///
/// 内部委托给 `tfind_impl`。测试模式下不生成此符号，
/// 测试通过 `tfind_impl` 直接调用，避免与 libc 的 `tfind` 符号冲突。
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn tfind(
    key: *const c_void,
    rootp: *mut *const c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void {
    unsafe { tfind_impl(key, rootp, compar) }
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

    /// 手动构建一个小型 AVL 树并用 tfind 查找。
    fn build_simple_tree() -> (*const c_void, *mut *const c_void) {
        // 构建树：
        //        root (key=50)
        //       /           \
        //   left (30)    right (70)
        let left = Box::into_raw(Box::new(Node {
            key: &30i32 as *const i32 as *const c_void,
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));
        let right = Box::into_raw(Box::new(Node {
            key: &70i32 as *const i32 as *const c_void,
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));
        let root = Box::into_raw(Box::new(Node {
            key: &50i32 as *const i32 as *const c_void,
            a: [left as *mut c_void, right as *mut c_void],
            h: 2,
        }));
        let root_const: *const c_void = root as *const c_void;
        let rootp = Box::into_raw(Box::new(root_const));
        (root as *const c_void, rootp as *mut *const c_void)
    }

    /// 清理手动构建的树节点。
    unsafe fn cleanup_tree(rootp: *mut *const c_void) {
        if rootp.is_null() {
            return;
        }
        let root = *rootp;
        if root.is_null() {
            let _ = Box::from_raw(rootp);
            return;
        }
        // 递归释放（简化处理：仅处理 2 层）
        let node = &*(root as *const Node);
        if !node.a[0].is_null() {
            let _ = Box::from_raw(node.a[0] as *mut Node);
        }
        if !node.a[1].is_null() {
            let _ = Box::from_raw(node.a[1] as *mut Node);
        }
        let _ = Box::from_raw(root as *mut Node);
        let _ = Box::from_raw(rootp);
    }

    test!("test_tfind_finds_existing" {
        // 测试 tfind 找到树中已存在的元素。
        unsafe {
            let (root_ptr, rootp) = build_simple_tree();
            let key: i32 = 50;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            // 应找到根节点
            assert_eq!(result, root_ptr as *mut c_void);
            cleanup_tree(rootp);
        }
    });

    test!("test_tfind_finds_left" {
        // 测试 tfind 找到左子树中的元素。
        unsafe {
            let (_root_ptr, rootp) = build_simple_tree();
            let key: i32 = 30;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            assert!(!result.is_null());
            cleanup_tree(rootp);
        }
    });

    test!("test_tfind_finds_right" {
        // 测试 tfind 找到右子树中的元素。
        unsafe {
            let (_root_ptr, rootp) = build_simple_tree();
            let key: i32 = 70;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            assert!(!result.is_null());
            cleanup_tree(rootp);
        }
    });

    test!("test_tfind_not_found" {
        // 测试 tfind 在不存在的 key 上返回 null。
        unsafe {
            let (_root_ptr, rootp) = build_simple_tree();
            let key: i32 = 99;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            assert_eq!(result, ptr::null_mut());
            cleanup_tree(rootp);
        }
    });

    test!("test_tfind_empty_tree" {
        // 测试 tfind 在空树（*rootp = null）上返回 null。
        unsafe {
            let root_const: *const c_void = ptr::null();
            let mut rootp: *const c_void = root_const;
            let key: i32 = 1;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                &mut rootp as *mut *const c_void,
                Some(cmp_int),
            );
            assert_eq!(result, ptr::null_mut());
        }
    });

    test!("test_tfind_null_rootp" {
        // 测试 tfind 在 null rootp 上返回 null。
        unsafe {
            let key: i32 = 1;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                ptr::null_mut(),
                Some(cmp_int),
            );
            assert!(result.is_null());
        }
    });

    test!("test_tfind_null_compar" {
        // 测试 tfind 使用 null compar（返回 null 而非崩溃）。
        unsafe {
            let (_root_ptr, rootp) = build_simple_tree();
            let key: i32 = 50;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                rootp,
                None,
            );
            assert!(result.is_null());
            cleanup_tree(rootp);
        }
    });

    test!("test_tfind_leaf_node" {
        // 测试 tfind 在叶节点上的查找。
        unsafe {
            let (_root_ptr, rootp) = build_simple_tree();
            let key: i32 = 30;
            let result = tfind_impl(
                &key as *const i32 as *const c_void,
                rootp,
                Some(cmp_int),
            );
            // 叶节点也是可找到的
            assert!(!result.is_null());
            cleanup_tree(rootp);
        }
    });

    test!("test_tfind_readonly" {
        // 测试 tfind 不修改 *rootp（只读保证）。
        unsafe {
            let (root_ptr, rootp) = build_simple_tree();
            let original_root = *rootp;
            let key: i32 = 50;
            tfind_impl(&key as *const i32 as *const c_void, rootp, Some(cmp_int));
            // *rootp 应保持不变
            assert_eq!(*rootp, original_root);
            cleanup_tree(rootp);
        }
    });
}