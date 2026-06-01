//! 共享类型定义 — 用于 search 模块的公共结构体、枚举和类型别名。

#![allow(dead_code, unused_imports)]

use core::ffi::{c_char, c_void};

/// 比较函数类型签名。接收两个 `*const c_void` 参数，返回负数/零/正数表示小于/等于/大于。
pub type CmpFn = unsafe extern "C" fn(*const c_void, *const c_void) -> i32;

/// POSIX hsearch 哈希表条目。
///
/// 包含一个以 null 结尾的字符串键和关联数据指针。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ENTRY {
    pub key: *mut c_char,
    pub data: *mut c_void,
}

/// hsearch 操作类型。
///
/// - `FIND`：仅查找，不修改表。
/// - `ENTER`：查找，若不存在则插入。
#[repr(C)]
pub enum ACTION {
    FIND = 0,
    ENTER = 1,
}

/// twalk 遍历顺序标记。
///
/// - `preorder`：前序遍历（访问节点前先访问左子树）。
/// - `postorder`：中序遍历（先访问左子树，再访问节点，再访问右子树）。
/// - `endorder`：后序遍历（访问节点后访问右子树）。
/// - `leaf`：叶节点（高度为 1 的节点）。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VISIT {
    preorder = 0,
    postorder = 1,
    endorder = 2,
    leaf = 3,
}

/// AVL 树内部节点（tsearch 族共享）。
///
/// 对应 C 源码 `tsearch.h` 中的 `struct node`。
/// - `key`：用户数据的指针（不复制数据）。
/// - `a`：左右子树指针数组，`a[0]` = 左子树，`a[1]` = 右子树。
/// - `h`：以该节点为根的子树高度。
#[repr(C)]
pub(crate) struct Node {
    pub(crate) key: *const c_void,
    pub(crate) a: [*mut c_void; 2],
    pub(crate) h: i32,
}

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    test!("test_entry_layout" {
        // 验证 ENTRY 的内存布局与 C 标准一致。
        assert_eq!(size_of::<ENTRY>(), size_of::<*mut c_void>() * 2);
        assert_eq!(align_of::<ENTRY>(), align_of::<*mut c_void>());
        // 验证字段偏移：key 在偏移 0，data 在偏移指针宽度处
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            assert_eq!(offset_of!(ENTRY, key), 0);
            assert_eq!(offset_of!(ENTRY, data), size_of::<*mut c_char>());
        }
    });

    test!("test_action_layout" {
        // 验证 ACTION 枚举大小与 C 一致（4 字节）。
        assert_eq!(size_of::<ACTION>(), size_of::<i32>());
        assert_eq!(ACTION::FIND as i32, 0);
        assert_eq!(ACTION::ENTER as i32, 1);
    });

    test!("test_visit_layout" {
        // 验证 VISIT 枚举大小与 C 一致（4 字节）。
        assert_eq!(size_of::<VISIT>(), size_of::<i32>());
        assert_eq!(VISIT::preorder as i32, 0);
        assert_eq!(VISIT::postorder as i32, 1);
        assert_eq!(VISIT::endorder as i32, 2);
        assert_eq!(VISIT::leaf as i32, 3);
    });

    test!("test_node_layout" {
        // 验证 Node 的内存布局与 C 源码 `struct node` 一致。
        // Node 包含：key(指针), a(2指针数组), h(i32) + 末尾填充对齐
        let ptr_size = size_of::<*const c_void>();
        let raw = ptr_size * 3 + size_of::<i32>();
        let align = align_of::<Node>();
        let expected = (raw + align - 1) & !(align - 1); // round up to alignment
        assert_eq!(size_of::<Node>(), expected);
        assert_eq!(align_of::<Node>(), ptr_size);
        // 验证字段偏移
        assert_eq!(offset_of!(Node, key), 0);
        assert_eq!(offset_of!(Node, a), ptr_size);
        assert_eq!(offset_of!(Node, h), ptr_size * 3);
    });

    test!("test_cmpfn_type" {
        // 验证 CmpFn 是与函数指针兼容的类型。
        unsafe extern "C" fn dummy(_a: *const c_void, _b: *const c_void) -> i32 {
            0
        }
        let _cmp: CmpFn = dummy;
        // 验证 CmpFn 大小与函数指针一致
        assert_eq!(size_of::<CmpFn>(), size_of::<usize>());
    });
}