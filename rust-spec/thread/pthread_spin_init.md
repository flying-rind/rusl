# pthread_spin_init — Rust 接口归约

## 原始 C 接口
```c
int pthread_spin_init(pthread_spinlock_t *s, int shared);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_spinlock_t 在 musl 中定义为 int
extern "C" fn pthread_spin_init(s: *mut core::ffi::c_int, shared: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
初始化一个自旋锁对象。`pthread_spinlock_t` 在 musl 中定义为 `int`，初始化值 0 表示"解锁/可用"状态。`shared` 参数被忽略（自旋锁仅在进程内有效）。

## 前置条件
- `s` 为非空指针（`!s.is_null()`），指向待初始化的自旋锁对象
- `shared` 为 `PTHREAD_PROCESS_PRIVATE` (0) 或 `PTHREAD_PROCESS_SHARED` (1)

## 后置条件
- `*s = 0`（锁处于解锁状态）
- 返回值为 0（成功）
- musl 中 `shared` 参数被忽略：自旋锁不支持跨进程共享，无论 shared 为何值，操作相同

## 不变量
无。

## 算法

```rust
// 将自旋锁值设为 0（解锁状态）
// 内部使用 AtomicI32 以支持后续原子操作，但对外 ABI 为 *mut c_int
pub extern "C" fn pthread_spin_init(s: *mut core::ffi::c_int, _shared: core::ffi::c_int) -> core::ffi::c_int {
    // unsafe: 解引用外部传入的裸指针
    // shared 参数在 musl 中被忽略
    unsafe { *s = 0; }
    0
}
```

对 C 调用者：
1. `extern "C" fn pthread_spin_init(s: *mut c_int, shared: c_int) -> c_int` 接收裸指针
2. 内部将 `*s` 设为 0（解锁状态），忽略 `shared` 参数
3. 返回 0

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering};

// 安全的 Rust 封装（供内部使用）
pub(crate) fn spin_init(lock: &AtomicI32) {
    lock.store(0, Ordering::Release);  // 原子写入 0, 保证后续锁操作可见
}
```

---

/* Rely */
[RELY]
Predefined Types:
  core::ffi::c_int                // 依赖1: C ABI 兼容的 int 类型
  core::sync::atomic::AtomicI32   // 依赖2: 原子 i32，供内部安全包装使用
Predefined Macros/Traits:
  (none)                          // 无内部依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_spin_init(s: *mut core::ffi::c_int, shared: core::ffi::c_int) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 pthread_spin_init 符号
Internal Interface:
  pub(crate) fn spin_init(lock: &core::sync::atomic::AtomicI32);
                                  // 安全包装，供 crate 内部使用
