# pthread_barrier_destroy -- Rust 接口归约

## 原始 C 接口
```c
int pthread_barrier_destroy(pthread_barrier_t *b);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_barrier_destroy(b: *mut pthread_barrier_t) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
销毁屏障对象。若屏障为进程共享（`_b_limit < 0`），且 `_b_lock` 非零（有线程正使用或等待重用屏障），则设置销毁标志并忙等全部线程退出。

## 前置条件
- `b` 为非空指针（`!b.is_null()`），指向有效的 `pthread_barrier_t` 对象
- 无任何线程正在调用 `pthread_barrier_wait` 使用该屏障（除了当前正在等待且即将被销毁过程排空的线程）

## 后置条件
- 始终返回 `0`
- **非进程共享屏障**（`_b_limit >= 0`）：
  - 立即返回，无任何清理操作（依赖调用者管理生命周期）
- **进程共享屏障**（`_b_limit < 0`）：
  - 若 `_b_lock != 0`（有等待者）：
    - 在 `_b_lock` 上原子 OR `i32::MIN`（`0x8000_0000`），标记"销毁中"
    - 忙等直到 `_b_lock` 的低 31 位降到 0（所有线程退出屏障）
    - 调用 `__vm_wait()` 等待所有内存映射操作完成，确保共享内存安全
  - 若 `_b_lock == 0`：立即返回

## 不变量
- 进程共享屏障销毁时必须确保无任何线程（可能在其他进程中）仍在使用该屏障
- `__vm_wait()` 调用确保所有共享内存映射的解映射操作已完成

## 算法
```rust
pub extern "C" fn pthread_barrier_destroy(b: *mut pthread_barrier_t) -> c_int {
    unsafe {
        let bar = &*b;
        if bar.b_limit() >= 0 {
            return 0;  // 非进程共享，无需清理
        }
        let lock_val = bar.b_lock().load(Ordering::Relaxed);
        if lock_val == 0 {
            return 0;  // 无使用者
        }
        // 设置销毁标志（i32::MIN = INT_MIN = 0x8000_0000）
        bar.b_lock().fetch_or(i32::MIN, Ordering::Release);
        // 忙等所有线程退出
        let mut v = bar.b_lock().load(Ordering::Acquire);
        while (v & i32::MAX) != 0 {
            __wait(bar.b_lock().as_ptr(), 0, v, 0);
            v = bar.b_lock().load(Ordering::Acquire);
        }
        __vm_wait();  // 确保 VM 操作完成（共享内存安全）
    }
    0
}
```

**注意**：`i32::MIN` 标志位用于 `pthread_barrier_wait` 中的 `pshared_barrier_wait` 函数识别销毁状态，配合自同步销毁安全的解锁逻辑（`v == INT_MIN + 1` 分支）。

## Rust 内部设计要点
- `PthreadBarrier` 提供 `b_limit()`, `b_lock()`, `b_count()`, `b_waiters()`, `b_waiters2()` 字段访问
- `fetch_or(i32::MIN, ...)` 替代 C 的 `a_or(&b->_b_lock, INT_MIN)`
- `i32::MAX` 用于提取低 31 位（与 `INT_MAX` 等效）
- `__wait()` 为 futex 等待，`__vm_wait()` 为 VM 锁等待，均定义于 `pthread_impl` 模块

---

/* Rely */
[RELY]
Predefined Types:
  pthread_barrier_t                // #[repr(C)] 屏障类型
  core::ffi::c_int                // Rust 核心库 C FFI 类型

Internal Module:
  pthread_impl::PthreadBarrier     // 字段访问方法（b_lock, b_limit 等）
  pthread_impl::__wait             // futex 等待
  pthread_impl::__vm_wait          // VM 锁等待，确保共享内存安全

Rust Core:
  core::sync::atomic::{Ordering, AtomicI32}
                                   // 原子操作，替代 C 的 a_or

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_barrier_destroy(b: *mut pthread_barrier_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
