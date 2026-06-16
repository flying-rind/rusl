# pthread_mutex_getprioceiling — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutex_getprioceiling(const pthread_mutex_t *restrict m, int *restrict ceiling);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutex_getprioceiling(
    m: *const pthread_mutex_t,
    ceiling: *mut core::ffi::c_int
) -> core::ffi::c_int;
```

---

## 意图
POSIX 定义此函数用于获取互斥锁的优先级天花板值。该功能依赖 `PTHREAD_PRIO_PROTECT` 协议（优先级保护），musl 不支持此协议，因此函数始终返回 `EINVAL` 以表明此功能不可用。Rust 实现保持相同行为。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `ceiling` 非空指针（`!ceiling.is_null()`）

## 后置条件
- Case 1（总是）：
  - 返回值为 `EINVAL`
  - `*ceiling` 不变

## 不变量
无。

## 算法
直接返回不支持：

```rust
extern "C" fn pthread_mutex_getprioceiling(
    _m: *const pthread_mutex_t,
    _ceiling: *mut core::ffi::c_int
) -> core::ffi::c_int {
    // musl 不支持 PTHREAD_PRIO_PROTECT，始终返回 EINVAL
    libc_errcode::EINVAL
}
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn mutex_getprioceiling(_m: &PthreadMutex) -> Result<core::ffi::c_int, core::ffi::c_int> {
    // 始终不支持
    Err(libc_errcode::EINVAL)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  libc_errcode::EINVAL            // 依赖: EINVAL 错误码常量

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutex_getprioceiling(m: *const pthread_mutex_t, ceiling: *mut core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_getprioceiling 符号
Internal Interface:
  pub(crate) fn mutex_getprioceiling(m: &PthreadMutex) -> Result<core::ffi::c_int, core::ffi::c_int>;
                                 // 安全包装，供 crate 内部使用
