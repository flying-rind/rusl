# sem_destroy — Rust 接口归约

## 原始 C 接口
```c
int sem_destroy(sem_t *sem);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_destroy(sem: *mut sem_t) -> core::ffi::c_int;
```

---

## 意图

销毁一个匿名信号量（通过 `sem_init` 创建）。在 rusl 实现中，匿名信号量无内核资源需要释放，故此函数为无操作，直接返回成功。

## 前置条件

- `sem` 指向之前通过 `sem_init` 成功初始化的信号量

## 后置条件

- 返回 `0`
- 信号量变为未初始化状态，后续使用行为未定义

## 不变量

- 始终返回 `0`（匿名信号量无需内核级资源释放）
- 调用者有责任确保无线程正在该信号量上阻塞

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部无需任何操作。

```rust
// extern "C" fn sem_destroy(sem: *mut sem_t) -> core::ffi::c_int {
//     0 // 直接返回成功，无操作
// }
```

注意：虽然当前 musl 实现为空操作（匿名信号量无内核资源），但 Rust 实现中可以考虑：
- 对 `sem_t` 内部字段做参数有效性的 debug_assert! 检查
- 使用 `core::mem::ManuallyDrop` 或类型状态模式在类型层面标记信号量的生命周期

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供 crate 内部使用）
pub(crate) fn sem_destroy_inner(sem: &mut Sem) {
    // 无操作（匿名信号量无内核资源释放）
    // 可选：将 Sem 内部状态标记为 destroyed 以辅助调试
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  sem_t                                              // 依赖1: 信号量类型定义
Predefined Macros/Traits:
  (无)                                                // 无操作

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_destroy(sem: *mut sem_t) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_destroy_inner(sem: &mut Sem);
