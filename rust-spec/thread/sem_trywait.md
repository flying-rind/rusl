# sem_trywait — Rust 接口归约

## 原始 C 接口
```c
int sem_trywait(sem_t *sem);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_trywait(sem: *mut sem_t) -> core::ffi::c_int;
```

---

## 意图

非阻塞递减（锁定）信号量。若信号量值为 0，立即返回 `EAGAIN` 而不等待。rusl 内部使用 Rust 的 `AtomicI32::compare_exchange_weak` 实现无锁 CAS 循环。

## 前置条件

- `sem` 为非空指针，指向有效 `sem_t`

## 后置条件

- Case 1 成功（信号量 > 0）：信号量计数值原子递减 1，返回 `0`
- Case 2 失败（信号量 == 0）：`errno = EAGAIN`，返回 `-1`

## 不变量

- 非阻塞操作，从不挂起调用线程
- CAS 循环确保并发安全（与 `sem_post` 等操作互不干扰）
- 仅当 `(val & SEM_VALUE_MAX) > 0` 时才尝试递减

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部使用纯 Rust 原子操作实现无锁递减。

```rust
// extern "C" 函数内部流程
// sem_trywait(sem):
//   1. 循环：
//      a. val = sem.count.load(Ordering::Acquire)
//      b. 若 val & SEM_VALUE_MAX == 0（计数值为 0）：
//         → errno = EAGAIN，返回 -1
//      c. new = val - 1
//      d. 若 sem.count.compare_exchange_weak(val, new, Ordering::Acquire, Ordering::Relaxed) 成功
//         → 返回 0
//      e. 否则（CAS 失败）→ 回到步骤 1a（重试，被其他线程并发修改）
```

内部改进要点：
- 使用 `AtomicI32::compare_exchange_weak` 替代 C 的 `a_cas` 宏
- 位掩码提取使用 Rust 的 `val & SEM_VALUE_MAX`
- 无需任何 `unsafe`（纯原子操作实现）

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 完全安全的 Rust 封装（纯原子操作，无需 unsafe）
pub(crate) fn sem_try_wait(sem: &SemInner) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SemInner                                           // 依赖1: 信号量内部结构（AtomicI32 count 字段）
  SEM_VALUE_MAX (0x7FFFFFFF)                         // 依赖2: 信号量最大值常量
Predefined Macros/Traits:
  core::sync::atomic::{AtomicI32, Ordering}          // 依赖3: Rust 原子操作（替代 C a_cas）

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_trywait(sem: *mut sem_t) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_try_wait(sem: &SemInner) -> Result<(), core::ffi::c_int>;
