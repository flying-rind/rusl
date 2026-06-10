# ofl_add 规约

## 复杂度分级: Level 1

> musl libc `__ofl_add` 的 Rust 实现 — 将新打开的 FILE 对象添加到全局打开文件链表。通常在 `fopen` / `fdopen` 等函数内部调用。

---

## 函数接口

```rust
// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 struct _IO_FILE

unsafe extern "C" fn __ofl_add(f: *mut FILE) -> *mut FILE;
```

[Visibility]: **Internal** — musl `hidden` 可见性，仅供内部模块（如 `fopen`、`fdopen`、`freopen` 等）调用。Rust 侧使用 `pub(crate)` 可见性，不对用户暴露。

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 有效的 `*mut FILE` 指针，指向新打开的、字段已正确初始化的流
- `f` 尚未在全局链表中（即 `(*f).next` 和 `(*f).prev` 尚未被设置或值无意义）
- 全局链表锁 `ofl_lock` 未被当前线程持有
- 调用方负责确保 `f` 的生命周期足够长（通常为 `'static` 或由调用方管理）

**[Post-condition]:**
- Case 1 `f != core::ptr::null_mut()`:
  - `f` 被插入全局链表头部
  - 链表指针调整：
    - `(*f).next` 指向插入前链表的头部（可能为 NULL）
    - 若原头部非空，原头部的 `(*head).prev` 指向 `f`
    - `(*f).prev` 为 `core::ptr::null_mut()`（头部节点）
    - 全局链表头 `ofl_head` 指向 `f`
  - 链表锁被释放
  - 返回 `f`（与输入相同，允许链式调用）

- Case 2 `f == core::ptr::null_mut()`:
  - 获取锁后直接释放，链表不变
  - 返回 `core::ptr::null_mut()`

**[Error Behavior]:**
- 本函数不产生错误。即使传入 NULL 指针也安全处理（锁后无操作返回 NULL）

---

## 不变量

**[Invariant]:**
- 全局打开文件链表始终是双向链表：若 `A.next == B` 且 `B != NULL`，则 `B.prev == A`
- 头部节点的 `prev` 始终为 `core::ptr::null_mut()`
- 尾部节点的 `next` 始终为 `core::ptr::null_mut()`
- 任何对链表的修改必须在持有 `ofl_lock` 的前提下进行
- 链表修改完成后锁必须被释放（无泄漏）
- 链表头 `ofl_head` 始终为最新插入的节点（头部插入策略）

---

## 意图

将新打开的 FILE 对象注册到全局链表中。该链表在程序退出时由 `__stdio_exit` 遍历，以刷新所有未写入的缓冲数据。返回值 `f` 允许调用方进行链式操作（如 `return __ofl_add(f)`）。

**关键设计点**：

- **头部插入策略**：始终将新流添加到链表头部，O(1) 时间，无需遍历
- **双向链表维护**：正确处理 `prev` 和 `next` 指针，确保 `prev` 指针可用于反向遍历（如 `fclose` 中的链表移除）
- **原子性**：链表修改在锁保护下完成，保证多线程安全
- **链式调用**：返回输入指针，符合 C 函数式编程惯例

Rust 侧实现要点：
- `__ofl_add` 为 `unsafe extern "C" fn`，保持与 C 侧调用约定完全兼容
- 函数内部使用 `__ofl_lock`/`__ofl_unlock` 获取/释放锁，确保链表操作原子性
- `FILE` 结构体的 `next`/`prev` 字段通过 `#[repr(C)]` 保证偏移量与原 C 完全一致
- Rust 侧内部实现可使用安全抽象（如将链表操作封装为内部安全函数），但对外接口必须保持 `unsafe extern "C"` ABI

---

## 系统算法

```
__ofl_add(f: *mut FILE) -> *mut FILE:
  1. head_ptr = __ofl_lock()            // 获取锁并取得 ofl_head 的地址
                                        // head_ptr: *mut *mut FILE
  2. let head = *head_ptr               // 读取当前链表头
  3. (*f).next = head                   // 新节点 next 指向原链表头
  4. if head != core::ptr::null_mut():
       (*head).prev = f                 // 原头的 prev 指向新节点
  5. *head_ptr = f                      // 更新链表头为新节点
  6. __ofl_unlock()                     // 释放锁
  7. return f                           // 返回输入指针
```

时间复杂度 O(1)。

---

## 依赖图

```
__ofl_add (Internal)
  ├── __ofl_lock    (see ofl spec) ──> *mut *mut FILE
  └── __ofl_unlock  (see ofl spec) ──> ()
```

---

## [RELY]

- `__ofl_lock()` — 获取全局文件链表锁并返回头指针地址（定义于 `ofl` 模块）
- `__ofl_unlock()` — 释放全局文件链表锁（定义于 `ofl` 模块）
- `FILE` 结构体定义 — 包含 `next: *mut FILE` 和 `prev: *mut FILE` 链表字段（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn __ofl_add(f: *mut FILE) -> *mut FILE;
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `__ofl_add`: 将新 FILE 对象插入全局打开文件链表头部，在锁保护下完成双向链表指针的原子调整，返回输入指针
- 该符号为 Internal 可见性，不对用户暴露，仅供内部模块使用
- 全局不变量保证：链表始终为双向链表，头部 `prev` 为 NULL，链表修改在锁保护下进行
