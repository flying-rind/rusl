# ofl 规约

## 复杂度分级: Level 1

> musl libc 全局打开文件链表（open file list）管理的 Rust 实现。提供加锁和解锁访问全局 FILE 链表的接口。该链表用于 `__stdio_exit` 在程序退出时遍历所有打开的流以刷新缓冲区。

---

## 接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 struct _IO_FILE

// __ofl_lock: 获取全局文件链表锁，返回链表头指针的地址
extern "C" fn __ofl_lock() -> *mut *mut FILE;

// __ofl_unlock: 释放全局文件链表锁
extern "C" fn __ofl_unlock();

// __stdio_ofl_lockptr: 指向链表锁的常量指针，用于 fork 后锁重置
// 对应 C 的 volatile int *const __stdio_ofl_lockptr = ofl_lock;
#[no_mangle]
static __stdio_ofl_lockptr: *mut c_int;
```

[Visibility]:
- `__ofl_lock` — **Internal**，musl `hidden` 可见性，仅供内部模块（如 `__ofl_add`、`__stdio_exit`、`fclose` 等）使用。Rust 侧使用 `pub(crate)` 可见性，不对外部用户暴露
- `__ofl_unlock` — **Internal**，与 `__ofl_lock` 配对使用，`pub(crate)` 可见性
- `__stdio_ofl_lockptr` — **Internal**，musl `hidden` 可见性，供 `fork` 路径中的 `__reinit_locks` 使用。Rust 侧使用 `pub(crate)` 可见性
- `ofl_head` — **Internal**，模块内部 `static mut` 变量，不对外导出
- `ofl_lock` — **Internal**，模块内部 `static mut` 变量，不对外导出

---

## 内部数据结构

### 1. `ofl_head: *mut FILE`

[Visibility]: Internal — 模块内部静态变量，不对外导出

```rust
// 全局打开文件链表头指针
static mut ofl_head: *mut FILE = core::ptr::null_mut();
```

全局打开文件链表的头指针。链表通过每个 `FILE` 对象的 `prev`/`next` 字段构成双向链表。新流通过 `__ofl_add` 添加到链表头部。

Rust 侧设计：
- 使用 `static mut` 或 `UnsafeCell<*mut FILE>` 确保可变性
- 初始值为 `core::ptr::null_mut()`（空链表）

### 2. `ofl_lock: c_int`

[Visibility]: Internal — 模块内部静态变量，不对外导出

```rust
// 保护 ofl_head 的自旋锁
static mut ofl_lock: c_int = 0;
```

保护 `ofl_head` 的自旋锁。使用 `LOCK`/`UNLOCK` 宏（底层调用 `__lock` / `__unlock`）进行操作。Rust 侧可封装为安全的锁抽象（如 `AtomicI32` + compare-and-swap 自旋锁）。

### 3. `__stdio_ofl_lockptr: *mut c_int`

```rust
// 指向 ofl_lock 的常量指针
#[no_mangle]
static __stdio_ofl_lockptr: *mut c_int = core::ptr::addr_of_mut!(ofl_lock);
```

[Visibility]: Internal — `#[no_mangle]` 导出符号以维持 C ABI 兼容性，Rust 侧 `pub(crate)`。

指向 `ofl_lock` 的常量指针。主要用于 `fork` 子进程中重置锁状态：`fork_impl` 中的 `__reinit_locks` 通过此指针将 `ofl_lock` 清零。

---

## 函数规约

### 4. `__ofl_lock`

```rust
extern "C" fn __ofl_lock() -> *mut *mut FILE;
```

[Visibility]: Internal — `pub(crate)`，musl `hidden` 可见性

#### Intent

获取全局打开文件链表锁，并返回链表头指针的地址（`*mut *mut FILE`）。调用方可通过返回的双重指针遍历或修改链表。调用方必须随后调用 `__ofl_unlock()` 释放锁。

该函数是链表的唯一入口点，任何需要遍历或修改全局 FILE 链表的代码必须经过此函数。

#### 前置条件

- `ofl_lock` 未被当前线程持有（禁止递归锁）

#### 后置条件

- `ofl_lock` 被当前线程持有（通过 `LOCK(ofl_lock)` 宏，Rust 侧使用自旋锁 `__lock`）
- 返回 `&raw mut ofl_head` 的指针（即 `ofl_head` 的地址），调用方可安全读取/修改链表
- 调用方必须在完成操作后调用 `__ofl_unlock()`

#### 系统算法

```
__ofl_lock() -> *mut *mut FILE:
  1. LOCK(&raw mut ofl_lock) — Rust 侧自旋锁获取
  2. return core::ptr::addr_of_mut!(ofl_head)
```

时间复杂度 O(1)。

---

### 5. `__ofl_unlock`

```rust
extern "C" fn __ofl_unlock();
```

[Visibility]: Internal — `pub(crate)`，musl `hidden` 可见性

#### Intent

释放全局打开文件链表锁。与 `__ofl_lock()` 配对使用。

#### 前置条件

- `ofl_lock` 被当前线程持有

#### 后置条件

- `ofl_lock` 被释放（通过 `UNLOCK(ofl_lock)` 宏，Rust 侧使用自旋锁 `__unlock`）

#### 系统算法

```
__ofl_unlock():
  1. UNLOCK(&raw mut ofl_lock) — Rust 侧自旋锁释放
```

时间复杂度 O(1)。

---

## 不变量

**[Invariant]:**
- 任何对 `ofl_head` 的读取或修改必须在持有 `ofl_lock` 的前提下进行
- `__ofl_lock()` 和 `__ofl_unlock()` 必须成对调用，不可嵌套
- `__stdio_ofl_lockptr` 始终指向 `ofl_lock`，用于 `fork` 后的锁重置
- `ofl_head` 在空链表时为 `core::ptr::null_mut()`
- 链表始终是双向链表：若 `A->next == B`，则 `B->prev == A`

---

## 意图

提供全局打开文件链表的同步访问机制。所有需要遍历或修改全局 FILE 链表的操作（如 `fclose` 移除节点、`__stdio_exit` 遍历刷新）均通过 `__ofl_lock`/`__ofl_unlock` 获取独占访问权。

Rust 侧实现要点：
- `ofl_head` 和 `ofl_lock` 为模块内部 `static mut`，不对外导出
- `__stdio_ofl_lockptr` 使用 `#[no_mangle]` 导出以维持 C ABI 兼容性，但 Rust 侧为 `pub(crate)`
- `LOCK`/`UNLOCK` 宏在 Rust 侧替代为 `__lock`/`__unlock` 函数调用或直接使用 `AtomicI32` 自旋锁实现
- `__ofl_lock` 和 `__ofl_unlock` 为 `extern "C" fn`，保持与 C 侧调用约定兼容（供 musl 内部跨模块调用）
- 返回类型 `*mut *mut FILE` 保持与原 C `FILE **` 一致，确保 ABI 兼容

---

## 依赖图

```
ofl
  ├── __ofl_lock (Internal) ──> LOCK(&ofl_lock) → __lock
  ├── __ofl_unlock (Internal) ──> UNLOCK(&ofl_lock) → __unlock
  ├── __stdio_ofl_lockptr (Internal) ──> 指向 ofl_lock
  ├── ofl_head (Internal, static) ──> *mut FILE, 链表头
  └── ofl_lock (Internal, static) ──> c_int, 自旋锁
```

---

## [RELY]

- `__lock` / `__unlock` — 自旋锁原语（定义于 `internal/lock` 模块）
- `FILE` 结构体定义 — 包含 `next`/`prev` 链表指针字段（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
extern "C" fn __ofl_lock() -> *mut *mut FILE;
extern "C" fn __ofl_unlock();
#[no_mangle] static __stdio_ofl_lockptr: *mut c_int;
```

本模块保证对外提供上述 ABI 兼容的函数和全局符号：
- `__ofl_lock`: 获取全局 FILE 链表锁，返回链表头指针的地址，调用方在锁保护下安全遍历/修改链表
- `__ofl_unlock`: 释放全局 FILE 链表锁，与 `__ofl_lock` 配对
- `__stdio_ofl_lockptr`: 指向锁变量的常量指针，供 `fork` 后子进程重置锁状态

三个符号均为 Internal 可见性，不对用户暴露，仅供 musl/rusl 内部模块使用。
