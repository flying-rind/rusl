# insque/remque Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn insque(element: *mut c_void, pred: *const c_void);

#[no_mangle]
pub unsafe extern "C" fn remque(element: *mut c_void);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- 元素的前两个指针大小字段分别被解释为 `next` 和 `prev`。
- `insque(element, pred)`: pred 为 null 时 element 成为独立节点。
- `remque(element)`: element 必须在链表中。

**[Post-condition]:**
- `insque`: element 被插入到 pred 之后（或成为独立节点）。
- `remque`: element 从链表摘除，邻居节点指针更新。

### 不变量

**[Invariant]:** 侵入式双向链表。用户结构体前 2 个指针字段 = {next, prev}。无内存分配。

### 意图

POSIX 风格侵入式双向链表操作。源自 VAX/VMS 系统指令。

### 系统算法

```
insque: element.next = pred.next; element.prev = pred; pred.next = element; ...
remque: if elem.prev { elem.prev.next = elem.next }; if elem.next { elem.next.prev = elem.prev }
```