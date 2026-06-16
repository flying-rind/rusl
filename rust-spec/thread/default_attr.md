# default_attr — Rust 接口归约

## 原始 C 接口

```c
// 模块内部全局变量（Internal, 不导出）
unsigned __default_stacksize = DEFAULT_STACK_SIZE;
unsigned __default_guardsize = DEFAULT_GUARD_SIZE;
```

---

## Rust 接口设计

### 内部状态（模块私有，不导出）

由于 `__default_stacksize` 和 `__default_guardsize` 在 musl 中标注为 Internal（不导出），仅在 `pthread_attr_init`、`pthread_setattr_default_np`、`pthread_getattr_default_np` 等函数中使用，在 Rust 实现中使用模块私有的内部状态管理。

```rust
// 模块内部静态变量（crate 内部可见，不对外导出）
// 使用 Rust 的同步原语包装，替代 C 中依赖 __inhibit_ptc / __release_ptc 的保护方式
pub(crate) fn default_stacksize() -> usize;
pub(crate) fn default_guardsize() -> usize;
pub(crate) fn set_default_stacksize(size: usize);
pub(crate) fn set_default_guardsize(size: usize);
```

内部实现可用 `core::sync::atomic::AtomicUsize` 或 `core::cell::UnsafeCell<usize>` 配合 PTC 锁机制管理读写一致性。

---

## 意图

保存进程全局的线程默认栈大小和守护页大小。初始值来源于编译期常量，后续可通过 `pthread_setattr_default_np` 单调递增修改（不可减小）。

## 前置条件

无（编译期常量初始化）。

## 后置条件

- `default_stacksize` 初始值为 `131072`（128KB = `DEFAULT_STACK_SIZE`）
- `default_guardsize` 初始值为 `8192`（8KB = `DEFAULT_GUARD_SIZE`）
- 后续修改需通过 PTC 同步机制保护
- 栈大小始终 >= 初始值，且 <= `DEFAULT_STACK_MAX`（8MB）
- 守护页大小始终 >= 初始值，且 <= `DEFAULT_GUARD_MAX`（1MB）

## 不变量

- 任何对默认值的修改必须在 PTC 同步保护下进行
- 值单调不减

## 算法

Rust 实现可将两个值存储为 `AtomicUsize`，读时使用 `Relaxed` 序（配合 PTC 锁已提供足够同步），写时使用 PTC 锁保护。由于这两个值仅在 PTC 锁保护的上下文修改，也可使用 `UnsafeCell<usize>` 配合锁手动管理，避免原子操作开销。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (无外部依赖 — 模块内部自包含)
Predefined Macros/Constants:
  DEFAULT_STACK_SIZE  = 131072   // 默认栈大小
  DEFAULT_GUARD_SIZE  = 8192     // 默认守护页大小
  DEFAULT_STACK_MAX   = 8388608  // 栈大小上限 (8MB)
  DEFAULT_GUARD_MAX   = 1048576  // 守护页大小上限 (1MB)
Synchronization:
  PTC 锁机制 (定义于 pthread_impl 模块) // 确保读写一致性

[GUARANTEE]
Exported Interface:
  (无对外导出接口 — 本模块仅提供 crate 内部函数)
Internal Interface:
  pub(crate) fn default_stacksize() -> usize;
  pub(crate) fn default_guardsize() -> usize;
  pub(crate) fn set_default_stacksize(size: usize);
  pub(crate) fn set_default_guardsize(size: usize);
  // 内部访问器，供 pthread_attr_init / pthread_setattr_default_np / pthread_getattr_default_np 使用
