//! twalk — AVL 二叉树遍历回调。
//! 对 C ABI 导出符号：`twalk`。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use super::types::{Node, VISIT};

/// 内部递归遍历函数（对应 C 源码 `walk`）。
///
/// - 叶节点（`h == 1`）：调用 `action(node, leaf, depth)` 一次。
/// - 内部节点：按 `preorder -> left子树 -> postorder -> right子树 -> endorder` 顺序调用三次。
unsafe fn walk(
    r: *const Node,
    action: &unsafe extern "C" fn(*const c_void, VISIT, i32),
    d: i32,
) {
    if r.is_null() {
        return;
    }
    if (*r).h == 1 {
        // 叶节点：调用一次 leaf 回调
        action(r as *const c_void, VISIT::leaf, d);
    } else {
        // 内部节点：三次回调
        action(r as *const c_void, VISIT::preorder, d);
        walk((*r).a[0] as *const Node, action, d + 1);
        action(r as *const c_void, VISIT::postorder, d);
        walk((*r).a[1] as *const Node, action, d + 1);
        action(r as *const c_void, VISIT::endorder, d);
    }
}

/// 遍历二叉树并对每个节点调用 `action` 回调。
///
/// 遍历顺序：
/// - 叶节点（`h == 1`）：调用 `action(node, leaf, depth)` 一次。
/// - 内部节点：调用三次 —— `preorder`（前序）、`postorder`（中序）、`endorder`（后序）。
/// - 根节点深度为 0。
///
/// `action` 的第一个参数是指向节点的指针（实际类型为 `*const Node`），调用者可通过此指针访问节点的 `key`。
///
/// # Safety
///
/// 调用者必须确保：
/// - `root` 为有效树根或 null。
/// - `action` 为有效回调函数或 None（此时无操作）。
/// - 在遍历过程中不修改树的结构。
#[no_mangle]
pub extern "C" fn twalk(
    root: *const c_void,
    action: Option<unsafe extern "C" fn(*const c_void, VISIT, i32)>,
) {
    if let Some(ref act) = action {
        unsafe { walk(root as *const Node, act, 0); }
    }
}
