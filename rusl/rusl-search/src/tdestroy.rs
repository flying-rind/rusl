//! tdestroy — 递归销毁整个 AVL 树并释放所有节点（GNU 扩展）。
//! 对 C ABI 导出符号：`tdestroy`。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use super::types::Node;

extern "C" {
    fn free(ptr: *mut c_void);
}

/// 递归销毁整棵二叉树。
///
/// 后序遍历树：对每个节点先递归销毁子树，然后调用 `free_key` 回调释放用户数据，最后释放节点内存。
///
/// - `root` 可为 null（空树，无操作）。
/// - `free_key` 可为 None（不释放用户数据）。
///
/// # Safety
///
/// 调用者必须确保：
/// - `root` 要么为 null，要么指向由 `tsearch`/`tdelete` 维护的有效树根节点。
/// - `free_key` 回调中不应访问树的结构。
/// - 销毁后所有指向树节点的指针变为悬空指针。
#[no_mangle]
pub unsafe extern "C" fn tdestroy(
    root: *mut c_void,
    free_key: Option<unsafe extern "C" fn(*mut c_void)>,
) {
    if root.is_null() {
        return;
    }
    let r = root as *mut Node;

    // 后序遍历：先递归处理子树，再释放当前节点
    if !(*r).a[0].is_null() {
        tdestroy((*r).a[0], free_key);
    }
    if !(*r).a[1].is_null() {
        tdestroy((*r).a[1], free_key);
    }

    // 调用 free_key 回调（如果提供）释放用户数据
    if let Some(fk) = free_key {
        fk((*r).key as *mut c_void);
    }

    // 释放节点内存
    free(r as *mut c_void);
}
