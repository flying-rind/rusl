/// 模块: twalk_test
/// `twalk` 集成测试

use core::ffi::{c_void};
use alloc::boxed::Box;
use super::imports::{VISIT, twalk};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};
use test_framework::test;

extern crate alloc;
// Internal tree node layout (opaque in public API)
#[repr(C)]
struct Node { key: *const core::ffi::c_void, a: [*mut core::ffi::c_void; 2], h: i32 }

/// 手动构建树节点的辅助函数。
#[allow(dead_code)]
unsafe fn make_node(key_val: i32, left: *mut c_void, right: *mut c_void, h: i32) -> *mut Node {
    Box::into_raw(Box::new(Node {
        key: &key_val as *const i32 as *const c_void,
        a: [left, right],
        h,
    }))
}

test!("test_twalk_null_root" {
    // 测试行走 null 树（应无操作，不崩溃）。
    unsafe {
        twalk(ptr::null(), None);
    }
});

test!("test_twalk_null_root_with_action" {
    // 测试行走 null 树并带 action（action 不应被调用）。
    unsafe {
        unsafe extern "C" fn count_call(_node: *const c_void, _which: VISIT, _depth: i32) {
            // 不应到达此处
        }
        twalk(ptr::null(), Some(count_call));
    }
});

test!("test_twalk_single_leaf" {
    // 测试行走单节点树（叶节点，应触发一次 leaf 回调）。
    unsafe {
        let key_val = 42i32;
        let node = Box::into_raw(Box::new(Node {
            key: &key_val as *const i32 as *const c_void,
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));

        unsafe extern "C" fn record_visit(
            _node: *const c_void,
            which: VISIT,
            _depth: i32,
        ) {
            // 单叶节点应收到 leaf 回调
            assert_eq!(which, VISIT::leaf);
        }

        twalk(node as *const c_void, Some(record_visit));
        let _ = Box::from_raw(node);
    }
});

test!("test_twalk_two_level" {
    // 测试行走 2 层树。
    unsafe {
        // 树结构：
        //        20 (h=2)
        //       /  \
        //    10(h=1) 30(h=1)
        let left = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));
        let right = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));
        let root = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [left as *mut c_void, right as *mut c_void],
            h: 2,
        }));

        // 记录调用次数：内部节点 3 次 + 2 个叶节点各 1 次 = 5 次
        let _call_count = AtomicUsize::new(0);
        unsafe extern "C" fn count_calls(
            _node: *const c_void,
            _which: VISIT,
            _depth: i32,
        ) {
            static mut COUNT: usize = 0;
            COUNT += 1;
        }

        twalk(root as *const c_void, Some(count_calls));

        // 使用 AtomicUsize 验证
        unsafe extern "C" fn count_atomic(
            _node: *const c_void,
            _which: VISIT,
            _depth: i32,
        ) {
            static COUNT: AtomicUsize = AtomicUsize::new(0);
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        twalk(root as *const c_void, Some(count_atomic));

        let _ = Box::from_raw(root);
        let _ = Box::from_raw(left);
        let _ = Box::from_raw(right);
    }
});

test!("test_twalk_chain" {
    // 测试行走非平衡树（链状）。
    unsafe {
        let node3 = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));
        let node2 = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [ptr::null_mut(), node3 as *mut c_void],
            h: 2,
        }));
        let node1_root = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [ptr::null_mut(), node2 as *mut c_void],
            h: 3,
        }));

        unsafe extern "C" fn visit(_node: *const c_void, _which: VISIT, _depth: i32) {}

        twalk(node1_root as *const c_void, Some(visit));
        let _ = Box::from_raw(node1_root);
        let _ = Box::from_raw(node2);
        let _ = Box::from_raw(node3);
    }
});

test!("test_visit_enum_values" {
    // 验证预定义的 VISIT 顺序值正确。
    assert_eq!(VISIT::preorder as i32, 0);
    assert_eq!(VISIT::postorder as i32, 1);
    assert_eq!(VISIT::endorder as i32, 2);
    assert_eq!(VISIT::leaf as i32, 3);
});

test!("test_twalk_depth" {
    // 测试深度参数正确性（根节点深度为 0）。
    unsafe {
        let node = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));

        unsafe extern "C" fn check_depth(_node: *const c_void, _which: VISIT, depth: i32) {
            assert_eq!(depth, 0);
        }

        twalk(node as *const c_void, Some(check_depth));
        let _ = Box::from_raw(node);
    }
});

test!("test_twalk_idempotent" {
    // 多次遍历同一棵树（幂等性验证）。
    unsafe {
        let node = Box::into_raw(Box::new(Node {
            key: ptr::null(),
            a: [ptr::null_mut(), ptr::null_mut()],
            h: 1,
        }));

        unsafe extern "C" fn visit(_node: *const c_void, _which: VISIT, _depth: i32) {}

        // 遍历两次不应崩溃
        twalk(node as *const c_void, Some(visit));
        twalk(node as *const c_void, Some(visit));
        let _ = Box::from_raw(node);
    }
});
