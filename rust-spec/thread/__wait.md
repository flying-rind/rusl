# __wait — Rust 接口归约

> rusl 内部 futex 等待函数。实现带自旋优化的 futex 等待：先在用户态自旋 100 次检查目标值变化，若未变化再进入内核态 futex 等待。Rust 实现中使用 `AtomicI32` 原子操作和 `core::hint::spin_loop` 替代 C 的 `a_spin()`。

## 原始 C 接口

```c
void __wait(volatile int *addr, volatile int *waiters, int val, int priv);
```

[Visibility]: Internal — 被 `pthread_impl.h` 声明，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __wait(
    addr: *mut core::ffi::c_int,
    waiters: *mut core::ffi::c_int,
    val: core::ffi::c_int,
    priv: core::ffi::c_int,
);
```

---

## 依赖图

```
__wait
  ├─> core::sync::atomic::AtomicI32      (原子操作)
  ├─> core::hint::spin_loop()            (用户态自旋)
  ├─> linux syscall: futex(FUTEX_WAIT)   (内核态等待)
  └─> linux syscall: futex(FUTEX_WAIT w/o FUTEX_PRIVATE) (回退)
```

---

## 函数规约

### 1. __wait

```rust
pub extern "C" fn __wait(
    addr: *mut core::ffi::c_int,
    waiters: *mut core::ffi::c_int,
    val: core::ffi::c_int,
    priv: core::ffi::c_int,
);
```

#### Intent

实现带自旋优化的 futex 等待原语。在进入昂贵的内核态 futex 等待之前，先在用户态自旋最多 100 次检查 `*addr` 是否发生变化。若 `*addr != val`（已被修改），则无需等待直接返回。此策略在锁竞争为中等强度时大幅减少系统调用开销。

可选地通过 `waiters` 参数维护等待者计数，方便调用者实现优化唤醒。

#### 前置条件

- `addr` 非空，指向有效的 futex 字
- `val` 为调用者期望的 `*addr` 当前值
- `waiters` 可为 NULL（不维护等待计数）或指向等待者计数器
- `priv` 为 0（进程共享）或非 0（进程私有）

#### 后置条件

- 返回时 `*addr != val`（值已改变）或已被正常唤醒
- 若 `waiters` 非空：等待开始时 `*waiters` 递增 1，返回前递减 1
- 自旋阶段（最多 100 次）：每次检查 `*addr == val`，若不等则立即返回；否则执行 `spin_loop()`
- 进入 futex 等待后：循环调用 `FUTEX_WAIT`，若返回 `-ENOSYS` 则回退到不带 `FUTEX_PRIVATE` 标志的调用

#### 系统算法

```
__wait(addr, waiters, val, priv):
  1. spins = 100
  2. if (priv != 0) priv = FUTEX_PRIVATE
  3. // 阶段一：用户态自旋
  4. while (spins > 0 && (waiters.is_null() || *waiters == 0)):
  5.   if (*addr == val): spin_loop(); spins -= 1
  6.   else: return                            // 值已变，无需等待
  7. // 阶段二：进入内核态等待
  8. if (!waiters.is_null()) atomic_inc(waiters) // 登记等待
  9. while (*addr == val):
 10.   futex(FUTEX_WAIT | priv, addr, val, 0)
 11.   if (result == -ENOSYS): futex(FUTEX_WAIT, addr, val, 0) // 回退
 12. if (!waiters.is_null()) atomic_dec(waiters) // 取消登记
```

#### 不变量

- 仅当 `*addr == val` 时才会阻塞在内核 futex 等待中
- 等待者计数（若启用）在进入 futex 等待前递增、退出后递减，始终保持一致
- 自旋仅在没有已知等待者时执行，避免在已有等待者时浪费 CPU

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering};
use core::hint;

/// 带自旋优化的 futex 等待。安全抽象封装了地址检查、自旋和 futex 系统调用。
pub(crate) fn futex_wait(
    addr: &AtomicI32,
    waiters: Option<&AtomicI32>,
    expected_val: i32,
    private: bool,
) {
    let mut spins = 100u32;
    let priv_flag = if private { FUTEX_PRIVATE } else { 0 };

    // 阶段一：用户态自旋
    if waiters.is_none() || waiters.unwrap().load(Ordering::Relaxed) == 0 {
        while spins > 0 {
            if addr.load(Ordering::Relaxed) == expected_val {
                hint::spin_loop();
                spins -= 1;
            } else {
                return;
            }
        }
    }

    // 阶段二：内核态 futex 等待
    if let Some(w) = waiters {
        w.fetch_add(1, Ordering::Relaxed);
    }
    while addr.load(Ordering::Relaxed) == expected_val {
        futex_wait_syscall(addr, FUTEX_WAIT | priv_flag, expected_val);
    }
    if let Some(w) = waiters {
        w.fetch_sub(1, Ordering::Relaxed);
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32          // 依赖1: 原子 I32 类型
  core::hint::spin_loop()                // 依赖2: 用户态自旋提示
  linux futex syscall (FUTEX_WAIT)       // 依赖3: futex 系统调用

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __wait(addr: *mut c_int, waiters: *mut c_int, val: c_int, priv: c_int);
                                         // 本模块保证对外提供与 C ABI 兼容的 __wait 符号
Internal Interface:
  pub(crate) fn futex_wait(addr: &AtomicI32, waiters: Option<&AtomicI32>, expected_val: i32, private: bool);
                                         // 安全包装，供 crate 内部使用
