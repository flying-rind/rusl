# sem_post — Rust 接口归约

## 原始 C 接口
```c
int sem_post(sem_t *sem);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_post(sem: *mut sem_t) -> core::ffi::c_int;
```

---

## 意图

释放/解锁信号量（V 操作），将计数值原子加 1。若存在等待线程则通过 futex wake 唤醒。rusl 内部使用 Rust 的 `AtomicI32::fetch_add` 和 CAS 循环实现无锁并发，仅在 futex 系统调用处使用 `unsafe`。

## 前置条件

- `sem` 为非空指针，指向有效 `sem_t`

## 后置条件

- Case 1 成功（计数值 < `SEM_VALUE_MAX`）：
  - 信号量计数值原子递增 1
  - 若存在等待者，通过 futex wake 唤醒
  - 返回 `0`
- Case 2 溢出（计数值已达 `SEM_VALUE_MAX`）：
  - `errno = EOVERFLOW`
  - 返回 `-1`

## 不变量

- 信号量计数值始终在 `0` 到 `SEM_VALUE_MAX`（0x7FFFFFFF）之间
- `sem.count` 的低 31 位为计数值，bit 31 为内部等待标记
- CAS 循环确保并发 `sem_post` / `sem_wait` 操作互不干扰

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部使用 Rust 原子操作实现无锁 CAS 循环，仅在 futex wake 系统调用处使用 `unsafe`。

```rust
// extern "C" 函数内部流程
// sem_post(sem):
//   1. priv = sem.flags（futex 私有/共享标志）
//   2. 循环（无锁 CAS）：
//      a. val = sem.count.load(Ordering::Acquire)
//      b. waiters = sem.waiters.load(Ordering::Acquire)
//      c. 若 (val & SEM_VALUE_MAX) == SEM_VALUE_MAX → errno = EOVERFLOW，返回 -1
//      d. new = val + 1
//      e. 若 waiters <= 1 → new &= !0x8000_0000（清除等待标记位 bit 31）
//      f. 若 sem.count.compare_exchange_weak(val, new, Ordering::Release, Ordering::Relaxed) 成功 → 跳出
//   3. 若 val < 0 || waiters > 0：
//      - 调用 unsafe { futex_wake(&sem.count, cnt, priv) }
//      - cnt = if waiters > 1 { 1 } else { i32::MAX }（决定唤醒数量）
//   4. 返回 0
```

内部改进要点：
- CAS 循环使用 `AtomicI32::compare_exchange_weak` 替代 C 的 `a_cas` 宏
- 位操作使用 Rust 的位运算：`val & 0x7FFF_FFFF`、`new & !0x8000_0000`
- futex `__wake` 系统调用包装为单个 `unsafe` 块调用

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（接收 &SemInner 引用，内部使用原子操作所以无需 &mut）
pub(crate) fn sem_post_inner(sem: &SemInner) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SemInner                                           // 依赖1: 信号量内部结构（AtomicI32 count + waiters + flags）
  futex_wake()                                       // 依赖2: futex wake 系统调用（unsafe 封装）
  SEM_VALUE_MAX (0x7FFFFFFF)                         // 依赖3: 信号量最大值常量
Predefined Macros/Traits:
  core::sync::atomic::{AtomicI32, Ordering}          // 依赖4: Rust 原子操作

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_post(sem: *mut sem_t) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_post_inner(sem: &SemInner) -> Result<(), core::ffi::c_int>;
