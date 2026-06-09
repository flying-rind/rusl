//! insque/remque — POSIX 侵入式双向链表操作。
//! 对 C ABI 导出符号：`insque`, `remque`。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;

/// 侵入式链表节点结构（与 C 源码 `struct node { struct node *next, *prev; }` 布局一致）。
#[repr(C)]
struct Link {
    next: *mut c_void,
    prev: *mut c_void,
}

/// 将 `element` 插入到 `pred` 之后。
///
/// `element` 的前两个指针字段被解释为 `next` 和 `prev` 指针，构成侵入式双向链表节点。
/// - 若 `pred` 非 null：`element` 被插入到 `pred` 节点之后。
/// - 若 `pred` 为 null：`element` 成为一个独立节点（next = prev = null）。
///
/// # Safety
///
/// 调用者必须确保：
/// - `element` 非空，且指向的内存块前两个指针字段位置可用作 `next`/`prev` 链接。
/// - 若 `pred` 非空，`pred` 必须是有效链表节点。
/// - 链表指针操作是侵入式的，不进行内存分配。
#[no_mangle]
pub extern "C" fn insque(element: *mut c_void, pred: *const c_void) {
    unsafe {
        let e = &mut *(element as *mut Link);
        if pred.is_null() {
            e.next = core::ptr::null_mut();
            e.prev = core::ptr::null_mut();
            return;
        }
        let p = &mut *(pred as *mut Link);
        e.next = p.next;              // e->next = p->next
        e.prev = pred as *mut c_void; // e->prev = p
        p.next = element;             // p->next = e
        if !e.next.is_null() {
            (*(e.next as *mut Link)).prev = element; // e->next->prev = e
        }
    }
}

/// 从双向链表中摘除 `element`。
///
/// 更新相邻节点的链接指针。被摘除节点的 next/prev 指针变为悬空状态。
///
/// # Safety
///
/// 调用者必须确保：
/// - `element` 处于有效链表中。
/// - `element` 非空。
/// - 摘除后不再通过旧的 neighbor 链接访问 `element`。
#[no_mangle]
pub extern "C" fn remque(element: *mut c_void) {
    unsafe {
        let e = &mut *(element as *mut Link);
        if !e.next.is_null() {
            (*(e.next as *mut Link)).prev = e.prev; // e->next->prev = e->prev
        }
        if !e.prev.is_null() {
            (*(e.prev as *mut Link)).next = e.next; // e->prev->next = e->next
        }
    }
}
