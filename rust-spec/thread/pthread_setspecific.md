# pthread_setspecific — Rust 接口归约

## 原始 C 接口
```c
int pthread_setspecific(pthread_key_t k, const void *x);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pthread_setspecific(k: pthread_key_t, x: *const core::ffi::c_void) -> core::ffi::c_int;
```

---

## 意图

将调用线程中键 `k` 关联的 TSD 值设置为 `x`。包含 COW（写时复制）优化：若新旧值相同则跳过写入，避免不必要的页表操作。rusl 内部使用 Safe Rust 实现，通过合理抽象避免不必要的裸指针操作。

## 前置条件

- `k` 是有效的 `pthread_key_t` 值（0 <= k < PTHREAD_KEYS_MAX）
- 调用线程的 TSD 数组已初始化

## 后置条件

- Case 1 新旧值相同：不执行任何写操作，不设置 `tsd_used` 标志，返回 `0`
- Case 2 新旧值不同：`tsd[k] = x`，`tsd_used = true`（标记需要 TSD 析构），返回 `0`

## 不变量

- 始终返回 `0`（POSIX 规定此函数不返回错误）
- `tsd_used` 标志保证线程退出时析构函数会被调用

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数，内部使用 Rust 安全抽象。

```rust
// extern "C" 函数内部流程（safe Rust）
// pthread_setspecific(k, x):
//   1. 获取当前线程的 TSD 数组可变引用及 tsd_used 标志
//   2. 若 tsd[k] != x：
//      a. tsd[k] = x
//      b. tsd_used = true
//   3. 返回 0
```

相比原始 C 实现的改进：
- 用 `Option<NonNull<c_void>>` 替代裸 `*const c_void` 提升类型安全
- TSD 数组使用 Rust 的 `UnsafeCell` 或线程局部 `RefCell` 管理
- COW 优化通过 Rust 的指针比较（`ptr::eq`）保留

pthread_setspecific(k, x):
1. 获取当前线程的 TSD 数组可变引用
2. 使用 `core::ptr::eq(tsd[k], x)` 进行等值比较（避免 COW）
3. 若不等：更新 `tsd[k]` 并设置 `tsd_used` 标志
4. 返回 `0`

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供 crate 内部使用）
pub(crate) fn setspecific(k: pthread_key_t, x: *const core::ffi::c_void) -> core::ffi::c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_self()                                   // 依赖1: 获取当前线程结构体（含 tsd 数组 + tsd_used 标志）
  PTHREAD_KEYS_MAX                                   // 依赖2: TSD 键最大数（128）
Predefined Macros/Traits:
  (无)                                                // 无需额外宏

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_setspecific(k: pthread_key_t, x: *const core::ffi::c_void) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn setspecific(k: pthread_key_t, x: *const core::ffi::c_void) -> core::ffi::c_int;
