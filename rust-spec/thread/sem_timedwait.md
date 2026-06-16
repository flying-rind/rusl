# sem_timedwait — Rust 接口归约

## 原始 C 接口
```c
int sem_timedwait(sem_t *restrict sem, const struct timespec *restrict at);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_timedwait(
    sem: *mut sem_t,
    at: *const core::ffi::timespec
) -> core::ffi::c_int;
```

当 `at` 为 `null` 时，等价于 `sem_wait`（无限等待）。

---

## 意图

递减（锁定）信号量（P 操作）。若信号量值为 0 则阻塞，直到信号量可用、超时或线程被取消。rusl 内部使用 Rust 原子操作实现 CAS 循环和自旋优化，futex 等待通过安全的系统调用封装执行。

## 前置条件

- `sem` 为非空指针，指向有效 `sem_t`
- `at` 可为 `null`（无限等待）或指向未来的绝对时间点（`CLOCK_REALTIME`）

## 后置条件

- Case 1 立即成功（信号量 > 0）：信号量计数值原子递减 1，返回 `0`
- Case 2 阻塞后成功（被 `sem_post` 唤醒）：信号量计数值原子递减 1，返回 `0`
- Case 3 超时：`errno = ETIMEDOUT`，返回 `-1`
- Case 4 线程被取消：调用清理函数递减等待者计数，线程不返回

## 不变量

- 等待线程被取消时，等待者计数通过清理机制保证正确递减
- 自旋优化（最多 100 次空转）减少短等待场景下的系统调用开销
- 信号量 bit 31 在进入等待前被 CAS 置为 1（标记有等待者）
- 阻塞等待使用 `CLOCK_REALTIME` 时钟

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部取消清理逻辑使用 Rust 的 RAII（`Drop` guard）替代 C 的 `pthread_cleanup_push/pop` 栈机制。

```rust
// 取消清理 guard（RAII 替代 pthread_cleanup_push/pop）
struct SemWaitGuard {
    waiters_ptr: *const AtomicI32,
}

impl Drop for SemWaitGuard {
    fn drop(&mut self) {
        // 原子递减等待者计数（替代 cleanup 函数）
        unsafe { &*self.waiters_ptr }.fetch_sub(1, Ordering::Release);
    }
}

// extern "C" 函数内部流程
// sem_timedwait(sem, at):
//   1. pthread_testcancel()
//   2. 快速路径：尝试 sem_trywait(sem)，若成功 → 返回 0
//   3. 自旋优化：最多 100 次 busy-wait
//      若期间信号量变为可用 → 回到步骤 2
//   4. while sem_trywait(sem) 失败：
//      a. 递增等待者计数：sem.waiters.fetch_add(1, Ordering::Release)
//      b. CAS 设置 bit 31 等待标记
//      c. 注册 SemWaitGuard（RAII 取消清理）
//      d. 调用 futex_timedwait(CLOCK_REALTIME, at, priv)
//      e. 若 futex 返回错误 → errno = r，返回 -1
//         （SemWaitGuard 在函数返回时自动 drop，递减等待者计数）
//      f. 循环回到 trywait
//   5. 返回 0
```

内部改进要点：
- **RAII 清理**：用 `SemWaitGuard`（实现 `Drop`）替代 `pthread_cleanup_push/pop`，在线程取消或函数返回时自动递减等待者计数
- 自旋使用 Rust 的 `core::hint::spin_loop()` 替代 C 的 `a_spin()`
- futex 系统调用封装在单个 `unsafe` 函数调用中
- CAS 循环使用 `AtomicI32::compare_exchange_weak`

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装
pub(crate) fn sem_timed_wait(
    sem: &SemInner,
    timeout: Option<&core::ffi::timespec>
) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SemInner                                           // 依赖1: 信号量内部结构（AtomicI32 字段）
  pthread_testcancel()                               // 依赖2: 检查取消点
  sem_trywait()                                      // 依赖3: 非阻塞递减（快速路径 + CAS 循环）
  futex_timedwait()                                  // 依赖4: futex 定时等待系统调用
  CLOCK_REALTIME                                     // 依赖5: 实时时钟常量
Predefined Macros/Traits:
  core::sync::atomic::{AtomicI32, Ordering}          // 依赖6: Rust 原子操作
  core::hint::spin_loop                              // 依赖7: 自旋提示（替代 C a_spin）
  core::ops::Drop                                    // 依赖8: RAII 清理（替代 pthread_cleanup_*）

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_timedwait(sem: *mut sem_t, at: *const core::ffi::timespec) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_timed_wait(sem: &SemInner, timeout: Option<&core::ffi::timespec>) -> Result<(), core::ffi::c_int>;
