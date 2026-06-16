# pthread_spin_trylock — Rust 接口归约

## 原始 C 接口
```c
int pthread_spin_trylock(pthread_spinlock_t *s);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_spinlock_t 在 musl 中定义为 int
extern "C" fn pthread_spin_trylock(s: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
尝试以非阻塞方式获取自旋锁。若锁空闲则获取并返回 0，若锁已被占用则立即返回错误而不等待。

## 前置条件
- `s` 为非空指针（`!s.is_null()`），指向通过 `pthread_spin_init` 初始化的自旋锁对象

## 后置条件
- Case 1 锁空闲（`*s == 0`）：CAS 原子地将 `*s` 从 0 设为 `EBUSY`，返回 0（旧值为 0 = 成功）
- Case 2 锁被占用（`*s != 0`）：CAS 失败，`*s` 不变，返回旧值即 `EBUSY`（非零值）
- 未获取到锁时调用线程不会阻塞

## 不变量
- 自旋锁状态仅在空闲（0）和占用（`EBUSY`）之间原子转换

## 算法

```rust
use core::sync::atomic::{AtomicI32, Ordering};

// 使用 Rust 原子 CAS 替代 C 的 a_cas
// 对外 ABI 仍保持 *mut c_int 兼容
pub extern "C" fn pthread_spin_trylock(s: *mut core::ffi::c_int) -> core::ffi::c_int {
    // unsafe: 将 *mut c_int 转换为 &AtomicI32 引用进行原子操作
    let lock = unsafe { &*(s as *const AtomicI32) };
    // 单次 CAS 尝试：若 *s == 0 则设为 EBUSY，返回旧值
    match lock.compare_exchange(0, EBUSY, Ordering::Acquire, Ordering::Relaxed) {
        Ok(_) => 0,     // 成功获取，旧值为 0
        Err(old) => old, // 锁被占用，返回旧值（EBUSY）
    }
}
```

对 C 调用者：
1. `extern "C" fn pthread_spin_trylock(s: *mut c_int) -> c_int` 接收裸指针
2. 内部执行单次原子 CAS：若 `*s == 0` 则设为 `EBUSY`
3. 返回旧值：0 = 成功，非零 = 锁已被占用

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering};

// 安全的 Rust 封装（供内部使用）
// 返回 true 表示成功获取锁，false 表示锁被占用
pub(crate) fn spin_trylock(lock: &AtomicI32) -> bool {
    lock.compare_exchange(0, EBUSY, Ordering::Acquire, Ordering::Relaxed).is_ok()
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  core::sync::atomic::AtomicI32   // 依赖1: Rust 原子 i32 类型
  core::sync::atomic::Ordering    // 依赖2: 内存顺序枚举
Predefined Constants:
  EBUSY                           // 依赖3: errno 值，用作锁状态标记（来自 <errno.h> 或内部常量定义）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_spin_trylock(s: *mut core::ffi::c_int) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 pthread_spin_trylock 符号
Internal Interface:
  pub(crate) fn spin_trylock(lock: &core::sync::atomic::AtomicI32) -> bool;
                                  // 安全包装，供 crate 内部使用
