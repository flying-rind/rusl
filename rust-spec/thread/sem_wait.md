# sem_wait — Rust 接口归约

## 原始 C 接口
```c
int sem_wait(sem_t *sem);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_wait(sem: *mut sem_t) -> core::ffi::c_int;
```

---

## 意图

阻塞递减（锁定）信号量。若信号量值为 0，则挂起当前线程直到信号量变为可用或被取消。直接委托给 `sem_timedwait(sem, null)`（无限等待）。

## 前置条件

- `sem` 为非空指针，指向有效 `sem_t`

## 后置条件

- Case 1 成功：信号量计数值原子递减 1，返回 `0`
- Case 2 线程被取消（取消点）：线程不返回，等待者计数正确递减（由 `sem_timedwait` 的 RAII 清理 guard 保证）

## 不变量

- 完全委托给 `sem_timedwait` 实现，`at == null` 时无超时
- 属于 POSIX 取消点（`sem_timedwait` 内部调用 `pthread_testcancel`）

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部纯委托。

```rust
// extern "C" 函数内部流程
// sem_wait(sem):
//   调用 sem_timedwait(sem, core::ptr::null()) → 返回其结果
```

等价关系说明：
- `sem_wait(sem)` 行为完全等价于 `sem_timedwait(sem, NULL)`
- 委托意味着 `sem_wait` 继承 `sem_timedwait` 的所有特性：取消点、自旋优化、RAII 清理等

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装
pub(crate) fn sem_wait_inner(sem: &SemInner) -> Result<(), core::ffi::c_int>;
// 内部实现: sem_timed_wait(sem, None)
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  sem_timedwait()                                    // 依赖1: 带超时的信号量递减函数（at == null 时无限等待）
  SemInner                                           // 依赖2: 信号量内部结构
Predefined Macros/Traits:
  (无)                                                // 纯委托

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_wait(sem: *mut sem_t) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_wait_inner(sem: &SemInner) -> Result<(), core::ffi::c_int>;
