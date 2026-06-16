# pthread_mutex_setprioceiling — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutex_setprioceiling(pthread_mutex_t *restrict m, int ceiling, int *restrict old);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutex_setprioceiling(
    m: *mut pthread_mutex_t,
    ceiling: core::ffi::c_int,
    old: *mut core::ffi::c_int
) -> core::ffi::c_int;
```

---

## 意图
POSIX 定义此函数用于设置互斥锁的优先级天花板并原子地加锁。该功能依赖 `PTHREAD_PRIO_PROTECT` 协议（优先级保护），musl 不支持此协议，因此函数始终返回 `EINVAL` 以表明此功能不可用。

根据 POSIX 规范，支持此函数的实现应在加锁前设置天花板、或以原子操作同时加锁并设置天花板。musl 选择完全不支持。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `old` 可以为空指针

## 后置条件
- Case 1（总是）：
  - 返回值为 `EINVAL`
  - 互斥锁状态不变
  - 若 `old` 非空，`*old` 不变

## 不变量
无。

## 算法
直接返回不支持：

```rust
extern "C" fn pthread_mutex_setprioceiling(
    _m: *mut pthread_mutex_t,
    _ceiling: core::ffi::c_int,
    _old: *mut core::ffi::c_int
) -> core::ffi::c_int {
    // musl 不支持 PTHREAD_PRIO_PROTECT，始终返回 EINVAL
    libc_errcode::EINVAL
}
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn mutex_setprioceiling(
    _m: &PthreadMutex,
    _ceiling: core::ffi::c_int
) -> Result<core::ffi::c_int, core::ffi::c_int> {
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
  extern "C" fn pthread_mutex_setprioceiling(m: *mut pthread_mutex_t, ceiling: core::ffi::c_int, old: *mut core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_setprioceiling 符号
Internal Interface:
  pub(crate) fn mutex_setprioceiling(m: &PthreadMutex, ceiling: core::ffi::c_int) -> Result<core::ffi::c_int, core::ffi::c_int>;
                                 // 安全包装，供 crate 内部使用
