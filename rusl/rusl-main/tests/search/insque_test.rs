/// 模块: insque_test
/// `insque` 集成测试

use core::ffi::{c_void};
use super::imports::{insque, remque};

use core::ptr;
use rusl_core::test;

/// 侵入式链表节点的辅助结构（与 C 源码 `struct node` 布局一致）。
#[repr(C)]
struct QueNode {
    next: *mut QueNode,
    prev: *mut QueNode,
}

test!("test_insque_after_pred" {
    // 测试将 element 插入到 pred 之后（链表中部）。
    unsafe {
        let mut head = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        let mut mid  = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        let mut tail = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };

        // 先构建 head -> tail
        head.next = &mut tail as *mut QueNode;
        tail.prev = &mut head as *mut QueNode;

        // 在 head 之后插入 mid：insque(&mid, &head)
        insque(
            &mut mid as *mut QueNode as *mut c_void,
            &head as *const QueNode as *const c_void,
        );

        assert_eq!(head.next, &mut mid as *mut QueNode);
        assert_eq!(mid.prev, &mut head as *mut QueNode);
        assert_eq!(mid.next, &mut tail as *mut QueNode);
        assert_eq!(tail.prev, &mut mid as *mut QueNode);
    }
});

test!("test_insque_at_end" {
    // 测试在链表末尾插入（pred 为 tail）。
    unsafe {
        let mut head = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        let mut node = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };

        insque(
            &mut node as *mut QueNode as *mut c_void,
            &head as *const QueNode as *const c_void,
        );

        assert_eq!(head.next, &mut node as *mut QueNode);
        assert_eq!(node.prev, &mut head as *mut QueNode);
        assert_eq!(node.next, ptr::null_mut());
    }
});

test!("test_insque_null_pred" {
    // 测试 pred 为 null 时 element 成为独立节点。
    unsafe {
        let mut node = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        insque(
            &mut node as *mut QueNode as *mut c_void,
            ptr::null(),
        );
        // 独立节点：next 和 prev 应为 null
        assert_eq!(node.next, ptr::null_mut());
        assert_eq!(node.prev, ptr::null_mut());
    }
});

test!("test_remque_middle" {
    // 测试从链表中间摘除节点。
    unsafe {
        let mut a = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        let mut b = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        let mut c = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };

        // a <-> b <-> c
        a.next = &mut b as *mut QueNode;
        b.prev = &mut a as *mut QueNode;
        b.next = &mut c as *mut QueNode;
        c.prev = &mut b as *mut QueNode;

        remque(&mut b as *mut QueNode as *mut c_void);

        // 摘除 b 后链表应为 a <-> c
        assert_eq!(a.next, &mut c as *mut QueNode);
        assert_eq!(c.prev, &mut a as *mut QueNode);
    }
});

test!("test_remque_head" {
    // 测试摘除链表头节点。
    unsafe {
        let mut head = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        let mut tail = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };

        head.next = &mut tail as *mut QueNode;
        tail.prev = &mut head as *mut QueNode;

        remque(&mut head as *mut QueNode as *mut c_void);

        // tail.prev 应为 null
        assert_eq!(tail.prev, ptr::null_mut());
    }
});

test!("test_remque_tail" {
    // 测试摘除链表尾节点。
    unsafe {
        let mut head = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        let mut tail = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };

        head.next = &mut tail as *mut QueNode;
        tail.prev = &mut head as *mut QueNode;

        remque(&mut tail as *mut QueNode as *mut c_void);

        // head.next 应为 null
        assert_eq!(head.next, ptr::null_mut());
    }
});

test!("test_remque_solo" {
    // 测试摘除链表中唯一节点。
    unsafe {
        let mut solo = QueNode { next: ptr::null_mut(), prev: ptr::null_mut() };
        remque(&mut solo as *mut QueNode as *mut c_void);
        // 摘除唯一节点不应崩溃
    }
});

test!("test_insque_chain" {
    // 测试连续 insque 构建链表并验证链接正确性。
    unsafe {
        let mut nodes: [QueNode; 5] = [
            QueNode { next: ptr::null_mut(), prev: ptr::null_mut() },
            QueNode { next: ptr::null_mut(), prev: ptr::null_mut() },
            QueNode { next: ptr::null_mut(), prev: ptr::null_mut() },
            QueNode { next: ptr::null_mut(), prev: ptr::null_mut() },
            QueNode { next: ptr::null_mut(), prev: ptr::null_mut() },
        ];

        // 逐个插入构建链表
        for i in 1..nodes.len() {
            insque(
                &mut nodes[i] as *mut QueNode as *mut c_void,
                &nodes[i - 1] as *const QueNode as *const c_void,
            );
        }

        // 验证双向链接
        for i in 0..nodes.len() - 1 {
            assert_eq!(nodes[i].next, &nodes[i + 1] as *const _ as *mut QueNode);
        }
        for i in 1..nodes.len() {
            assert_eq!(nodes[i].prev, &nodes[i - 1] as *const _ as *mut QueNode);
        }
    }
});
