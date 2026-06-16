# pthread_spin_lock — Rust 接口归约

## 原始 C 接口
```c
int pthread_spin_lock(pthread_spinlock_t *s);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_spinlock_t 在 musl 中定义为 int，锁占用标记为 EBUSY
extern "C" fn pthread_spin_lock(s: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
以忙等待（自旋）方式获取自旋锁。若锁已被其他线程持有，调用线程在循环中反复检查直到锁可用，不进入内核睡眠。

## 前置条件
- `s` 为非空指针（`!s.is_null()`），指向通过 `pthread_spin_init` 初始化的自旋锁对象
- 调用线程当前未持有该自旋锁（自旋锁不可重入）

## 后置条件
- Case 1 锁空闲（`*s == 0`）：CAS 原子地将 `*s` 从 0 设为 `EBUSY`，返回 0，调用线程获得锁
- Case 2 锁被占用：在循环中反复执行自旋等待（`spin_loop` 提示 CPU）直到获取成功
- 锁值设为 `EBUSY`（非零）而非传统 1，以便与 `EAGAIN`/`EWOULDBLOCK` 等 errno 值区分

## 不变量
- 自旋锁为"独占访问"锁：同一时刻最多一个线程能通过 CAS 成功将 `*s` 从 0 改为非零

## 算法

```rust
use core::sync::atomic::{AtomicI32, Ordering};
use core::hint;

// 使用 Rust 原子类型替代 C 的 a_cas / a_spin
// 对外 ABI 仍保持 *mut c_int 兼容
pub extern "C" fn pthread_spin_lock(s: *mut core::ffi::c_int) -> core::ffi::c_int {
    // unsafe: 将 *mut c_int 转换为 &AtomicI32 引用
    // 需要通过 AtomicI32 进行原子操作
    let lock = unsafe { &*(s as *const AtomicI32) };
    // 自旋等待：反复尝试 CAS(0 -> EBUSY)
    while lock.compare_exchange(0, EBUSY, Ordering::Acquire, Ordering::Relaxed).is_err() {
        hint::spin_loop();  // CPU 放松/暂停，等同于 a_spin()
    }
    0
}
```

对 C 调用者：
1. `extern "C" fn pthread_spin_lock(s: *mut c_int) -> c_int` 接收裸指针
2. 内部通过 `AtomicI32::compare_exchange` 在循环中尝试 CAS
3. 每次失败后调用 `hint::spin_loop()` 降低 CPU 争用
4. 返回 0 表示成功获取锁

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering};
use core::hint;

// 安全的 Rust 封装（供内部使用）
pub(crate) fn spin_lock(lock: &AtomicI32) {
    while lock.compare_exchange(0, EBUSY, Ordering::Acquire, Ordering::Relaxed).is_err() {
        hint::spin_loop();
    }
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  core::sync::atomic::AtomicI32   // 依赖1: Rust 原子 i32 类型
  core::sync::atomic::Ordering    // 依赖2: 内存顺序枚举
  core::hint::spin_loop           // 依赖3: CPU 自旋提示指令（替代 a_spin）
Predefined Constants:
  EBUSY                           // 依赖4: errno 值，用作锁"已占用"标记（来自 <errno.h> 或内部常量定义）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_spin_lock(s: *mut core::ffi::c_int) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 pthread_spin_lock 符号
Internal Interface:
  pub(crate) fn spin_lock(lock: &core::sync::atomic::AtomicI32);
                                  // 安全包装，供 crate 内部使用
