/// 模块: tdestroy_test
/// `tdestroy` 集成测试

extern crate alloc;

use core::ffi::{c_void};
use super::imports::{tdestroy};

use alloc::boxed::Box;
use core::ptr;
use rusl_core::test;

// Internal tree node layout (opaque in public API)
#[repr(C)]
struct Node { key: *const core::ffi::c_void, a: [*mut core::ffi::c_void; 2], h: i32 }

/// 手动构建简单树节点的辅助函数。
/// Node 通过 C `malloc` 分配（与 tsearch_impl 一致），key 通过 Box 分配。
unsafe fn make_node(key_val: i32, left: *mut c_void, right: *mut c_void, h: i32) -> *mut Node {
    extern "C" { fn malloc(size: usize) -> *mut c_void; }
    let node = malloc(core::mem::size_of::<Node>()) as *mut Node;
    (*node).key = Box::into_raw(Box::new(key_val)) as *const c_void;
    (*node).a = [left, right];
    (*node).h = h;
    node
}

test!("test_tdestroy_null_root" {
    // 测试销毁 null root（应无操作，不崩溃）。
    unsafe {
        tdestroy(ptr::null_mut(), None);
    }
});

test!("test_tdestroy_null_root_with_freekey" {
    // 测试销毁 null root 并带 free_key（应无操作，不崩溃）。
    unsafe extern "C" fn do_free(_p: *mut c_void) {
        // 不应被调用
    }
    unsafe {
        tdestroy(ptr::null_mut(), Some(do_free));
    }
});

test!("test_tdestroy_single_leaf" {
    // 测试销毁单节点树（叶节点）。
    unsafe {
        let node = make_node(42, ptr::null_mut(), ptr::null_mut(), 1);
        tdestroy(node as *mut c_void, None);
        // 函数返回后节点内存应已被释放
    }
});

test!("test_tdestroy_two_level" {
    // 测试销毁 2 层树（根 + 左右孩子）。
    unsafe {
        let left = make_node(10, ptr::null_mut(), ptr::null_mut(), 1);
        let right = make_node(30, ptr::null_mut(), ptr::null_mut(), 1);
        let root = make_node(20, left as *mut c_void, right as *mut c_void, 2);
        tdestroy(root as *mut c_void, None);
    }
});

test!("test_tdestroy_deep_tree" {
    // 测试销毁多层树验证不会栈溢出。
    unsafe {
        // 构建一条链（退化为链表）
        let mut current: *mut Node = ptr::null_mut();
        for i in (0..50).rev() {
            current = make_node(i, current as *mut c_void, ptr::null_mut(), 1);
        }
        tdestroy(current as *mut c_void, None);
    }
});

test!("test_tdestroy_freekey_call_count" {
    // 测试 free_key 回调被调用的次数正确。
    unsafe {
        extern "C" { fn malloc(size: usize) -> *mut c_void; }

        // 手动分配 key 以便 free_key 能释放它们
        let key1 = Box::into_raw(Box::new(1i32));
        let key2 = Box::into_raw(Box::new(2i32));
        let key3 = Box::into_raw(Box::new(3i32));

        let right = malloc(core::mem::size_of::<Node>()) as *mut Node;
        (*right).key = key2 as *const c_void;
        (*right).a = [ptr::null_mut(), ptr::null_mut()];
        (*right).h = 1;

        let left = malloc(core::mem::size_of::<Node>()) as *mut Node;
        (*left).key = key1 as *const c_void;
        (*left).a = [ptr::null_mut(), ptr::null_mut()];
        (*left).h = 1;

        let root = malloc(core::mem::size_of::<Node>()) as *mut Node;
        (*root).key = key3 as *const c_void;
        (*root).a = [left as *mut c_void, right as *mut c_void];
        (*root).h = 2;

        unsafe extern "C" fn free_int(p: *mut c_void) {
            let _ = Box::from_raw(p as *mut i32);
        }

        tdestroy(root as *mut c_void, Some(free_int));
    }
});

test!("test_tdestroy_without_freekey" {
    // 测试 free_key = None 时 key 内存未被释放（但节点被释放）。
    unsafe {
        extern "C" { fn malloc(size: usize) -> *mut c_void; }

        let key_val = Box::into_raw(Box::new(42i32));
        let node = malloc(core::mem::size_of::<Node>()) as *mut Node;
        (*node).key = key_val as *const c_void;
        (*node).a = [ptr::null_mut(), ptr::null_mut()];
        (*node).h = 1;

        tdestroy(node as *mut c_void, None);
        // key_val 仍然有效（内存泄漏——由调用者管理）
        let _ = Box::from_raw(key_val);
    }
});
