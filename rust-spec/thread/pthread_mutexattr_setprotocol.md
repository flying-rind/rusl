# pthread_mutexattr_setprotocol — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutexattr_setprotocol(pthread_mutexattr_t *a, int protocol);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutexattr_setprotocol(
    a: *mut pthread_mutexattr_t,
    protocol: core::ffi::c_int
) -> core::ffi::c_int;
```

---

## 类型与常量

| 常量 | 值 | 含义 |
|------|-----|------|
| `PTHREAD_PRIO_NONE` | 0 | 无优先级协议 |
| `PTHREAD_PRIO_INHERIT` | 1 | 优先级继承（PI） |
| `PTHREAD_PRIO_PROTECT` | 2 | 优先级保护（musl 不支持） |

PI 标志存储在 `__attr` 的位 3（`__attr & 8`）。

---

## 意图
设置互斥锁属性对象的优先级协议。支持 `PTHREAD_PRIO_NONE`（无协议）和 `PTHREAD_PRIO_INHERIT`（优先级继承）两种模式。优先级继承可使高优先级线程在等待低优先级线程持有的互斥锁时，临时提升低优先级线程的优先级，从而缓解优先级反转。

## 前置条件
- `a` 非空指针（`!a.is_null()`）
- `a` 指向一个已初始化的 `pthread_mutexattr_t`

## 后置条件
- Case 1 `protocol` 为 `PTHREAD_PRIO_NONE`(0)：
  - `(*a).__attr` 的位 3 被清除
  - 返回值为 `0`
- Case 2 `protocol` 为 `PTHREAD_PRIO_INHERIT`(1)，且内核支持 PI futex：
  - `(*a).__attr` 的位 3 被设置
  - 返回值为 `0`
- Case 3 `protocol` 为 `PTHREAD_PRIO_INHERIT`(1)，但内核不支持 PI futex：
  - `(*a).__attr` 不变
  - 返回值为内核返回的 `errno`（如 `ENOSYS`）
- Case 4 `protocol` 为 `PTHREAD_PRIO_PROTECT`(2)：
  - `(*a).__attr` 不变
  - 返回值为 `ENOTSUP`
- Case 5 `protocol` 为其他非法值：
  - `(*a).__attr` 不变
  - 返回值为 `EINVAL`

## 不变量
- PI 内核支持检测结果在内部惰性缓存（首次检测后不变）
- 非 PI 属性位（type、robust、process-shared 位）不受此操作影响
- 用于探测 PI 支持的 futex 锁为局部变量，不会被实际修改

## 算法
内部按 protocol 值分派：

```rust
extern "C" fn pthread_mutexattr_setprotocol(
    a: *mut pthread_mutexattr_t,
    protocol: core::ffi::c_int
) -> core::ffi::c_int {
    // 内部使用 Rust 安全抽象：
    // 1. 惰性检测内核 PI 支持（通过一次 futex FUTEX_LOCK_PI 探测）
    // 2. 结果缓存在模块级别的 AtomicI32 中
    // 3. 对属性位掩码进行位操作
}
```

内部实现可复用模块级静态变量惰性缓存 PI 检测结果。探测逻辑通过 `linux_futex` 内部的 `FUTEX_LOCK_PI` 操作完成。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn mutexattr_setprotocol(
    a: &mut pthread_mutexattr_t,
    protocol: MutexProtocol
) -> Result<(), core::ffi::c_int>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MutexProtocol {
    None = 0,
    Inherit = 1,
    Protect = 2,
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  linux_futex (内部模块)         // 依赖: futex 系统调用封装，用于探测 PI 支持
  core::sync::atomic::AtomicI32  // 依赖: 惰性缓存 PI 检测结果

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutexattr_setprotocol(a: *mut pthread_mutexattr_t, protocol: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutexattr_setprotocol 符号
Internal Interface:
  pub(crate) fn mutexattr_setprotocol(a: &mut pthread_mutexattr_t, protocol: MutexProtocol) -> Result<(), core::ffi::c_int>;
                                 // 安全包装，使用枚举避免非法值，供 crate 内部使用
  pub(crate) enum MutexProtocol;
                                 // 安全抽象，替代 C 裸 int
