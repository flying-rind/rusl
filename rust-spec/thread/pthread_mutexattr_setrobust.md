# pthread_mutexattr_setrobust — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutexattr_setrobust(pthread_mutexattr_t *a, int robust);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutexattr_setrobust(
    a: *mut pthread_mutexattr_t,
    robust: core::ffi::c_int
) -> core::ffi::c_int;
```

---

## 类型与常量

| 常量 | 值 | 含义 |
|------|-----|------|
| `PTHREAD_MUTEX_STALLED` | 0 | 非健壮（默认） |
| `PTHREAD_MUTEX_ROBUST` | 1 | 健壮互斥锁 |

健壮标志存储在 `__attr` 的位 2（`__attr & 4`）。

---

## 意图
设置互斥锁属性对象的健壮性标志。当 `PTHREAD_MUTEX_ROBUST` 被设置时，若互斥锁的持有者线程异常终止，后续加锁操作可检测到此状态并恢复。

## 前置条件
- `a` 非空指针（`!a.is_null()`）
- `a` 指向一个已初始化的 `pthread_mutexattr_t`

## 后置条件
- Case 1 `robust` 为 `PTHREAD_MUTEX_STALLED`(0)：
  - `(*a).__attr` 的位 2 被清除（`__attr &= !4`）
  - 返回值为 `0`
- Case 2 `robust` 为 `PTHREAD_MUTEX_ROBUST`(1)，且内核支持 robust list：
  - `(*a).__attr` 的位 2 被设置（`__attr |= 4`）
  - 返回值为 `0`
- Case 3 `robust` 为 `PTHREAD_MUTEX_ROBUST`(1)，但内核不支持 robust list：
  - `(*a).__attr` 不变
  - 返回值为内核返回的 `errno`（如 `ENOSYS`）
- Case 4 `robust > 1`（非法值）：
  - `(*a).__attr` 不变
  - 返回值为 `EINVAL`

## 不变量
- 内核 robust list 支持检测结果在内部惰性缓存（首次检测后不变）
- 非 robust 属性位（type、PI、process-shared 位）不受此操作影响

## 算法
内部按 robust 值分派，惰性检测内核 robust list 支持：

```rust
extern "C" fn pthread_mutexattr_setrobust(
    a: *mut pthread_mutexattr_t,
    robust: core::ffi::c_int
) -> core::ffi::c_int {
    // 内部使用 Rust 安全抽象：
    // 1. robust > 1 → 返回 EINVAL
    // 2. robust == 0 → 清除位 2
    // 3. robust == 1 → 惰性检测内核 robust list 支持（SYS_get_robust_list）
    //    结果缓存在模块级 AtomicI32 中
}
```

内部实现可复用模块级静态变量惰性缓存 robust list 检测结果。探测逻辑通过 `linux_get_robust_list` 系统调用完成。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Robust {
    Stalled = 0,
    Robust = 1,
}

pub(crate) fn mutexattr_setrobust(
    a: &mut pthread_mutexattr_t,
    robust: Robust
) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  linux_get_robust_list (内部模块)  // 依赖: 系统调用封装，用于探测 robust list 支持
  core::sync::atomic::AtomicI32     // 依赖: 惰性缓存检测结果

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutexattr_setrobust(a: *mut pthread_mutexattr_t, robust: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutexattr_setrobust 符号
Internal Interface:
  pub(crate) fn mutexattr_setrobust(a: &mut pthread_mutexattr_t, robust: Robust) -> Result<(), core::ffi::c_int>;
                                 // 安全包装，使用枚举替代裸 int，供 crate 内部使用
  pub(crate) enum Robust;
                                 // 健壮性标志枚举
