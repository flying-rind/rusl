# pthread_spin_unlock — Rust 接口归约

## 原始 C 接口
```c
int pthread_spin_unlock(pthread_spinlock_t *s);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_spinlock_t 在 musl 中定义为 int
extern "C" fn pthread_spin_unlock(s: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
释放自旋锁，使其可被其他等待线程获取。使用原子写操作保证对锁状态的修改对其他线程可见。

## 前置条件
- `s` 为非空指针（`!s.is_null()`），指向通过 `pthread_spin_init` 初始化的自旋锁对象
- 调用线程当前持有该自旋锁（即此前已通过 `pthread_spin_lock` 或 `pthread_spin_trylock` 成功获取）

## 后置条件
- `*s = 0`（解锁状态），通过原子存储写入以保证内存可见性
- 返回 0（成功）
- 其他自旋等待此锁的线程可以观察到锁变为空闲

## 不变量
- 解锁操作总是成功，不会失败

## 算法

```rust
use core::sync::atomic::{AtomicI32, Ordering};

// 使用 Rust 原子 store 替代 C 的 a_store
// 对外 ABI 仍保持 *mut c_int 兼容
pub extern "C" fn pthread_spin_unlock(s: *mut core::ffi::c_int) -> core::ffi::c_int {
    // unsafe: 将 *mut c_int 转换为 &AtomicI32 引用进行原子操作
    let lock = unsafe { &*(s as *const AtomicI32) };
    lock.store(0, Ordering::Release);  // 原子写入 0，释放语义
    0
}
```

对 C 调用者：
1. `extern "C" fn pthread_spin_unlock(s: *mut c_int) -> c_int` 接收裸指针
2. 内部通过 `AtomicI32::store(0, Release)` 原子写入 0，保证释放语义
3. 返回 0 表示成功

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering};

// 安全的 Rust 封装（供内部使用）
// Release 顺序保证临界区内的所有内存写入在解锁前对其他线程可见
pub(crate) fn spin_unlock(lock: &AtomicI32) {
    lock.store(0, Ordering::Release);
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  core::sync::atomic::AtomicI32   // 依赖1: Rust 原子 i32 类型
  core::sync::atomic::Ordering    // 依赖2: 内存顺序枚举（Release）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_spin_unlock(s: *mut core::ffi::c_int) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 pthread_spin_unlock 符号
Internal Interface:
  pub(crate) fn spin_unlock(lock: &core::sync::atomic::AtomicI32);
                                  // 安全包装，供 crate 内部使用
