# call_once — Rust 接口归约

## 原始 C 接口
```c
void call_once(once_flag *flag, void (*func)(void));
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.2.1)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn call_once(flag: *mut core::ffi::c_int, func: Option<extern "C" fn()>);
```

---

## 意图
确保 `func` 在由同一 `once_flag` 控制的多次 `call_once` 调用中恰好执行一次。是内部 `__pthread_once` 的纯转发包装器。

## 前置条件
- `flag` 为非空指针（`!flag.is_null()`），指向通过 `ONCE_FLAG_INIT` (值为 `0`) 初始化的 `once_flag` 对象
- `func` 不为 `None`，指向无参数、无返回值的有效函数
- `flag` 的生命周期覆盖所有使用它的 `call_once` 调用

## 后置条件
- Case 1 `func` 尚未执行过：`func` 被调用一次，之后 `*flag` 标记为非零（已完成）。所有并发或后续 `call_once` 调用将阻塞等待完成或立即返回
- `func` 在所有线程中恰好执行一次（即使并发调用）
- 若 `func` 未执行完时另有线程调用 `call_once`：该线程阻塞等待直到 `func` 执行完毕
- 函数无返回值

## 不变量
- 同一 `once_flag` 全局最多仅执行一次 `func`
- `ONCE_FLAG_INIT` = `0`，非零表示已完成

## 算法
`call_once` 保持简单的委托模式：将所有逻辑转发给内部的 POSIX `__pthread_once` 实现：

```rust
extern "C" fn call_once(flag: *mut c_int, func: Option<extern "C" fn()>) {
    // 内部委托给 pthread 模块的 __pthread_once
    // unsafe 仅限于 FFI 调用
    unsafe { __pthread_once(flag, func); }
}
```

内部 `__pthread_once` 由 `pthread_once` 模块提供（见 `pthread_impl` spec），使用原子操作（`a_cas`/`a_swap`）和 futex（`__wait`/`__wake`）实现多线程安全的一次性执行。该内部函数不需要对 C 调用者暴露，作为 `pub(crate)` 函数即可。

---

## Rust 内部辅助接口（模块私有）

```rust
// 安全的 C11 once_flag 抽象（仅供内部使用）
pub(crate) struct OnceFlag {
    inner: core::sync::atomic::AtomicI32,
}

impl OnceFlag {
    pub(crate) const fn new() -> Self;
    pub(crate) fn call_once(&self, f: fn());
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_once(flag: *mut c_int, func: Option<extern "C" fn()>)  // 依赖1: musl 内部 pthread_once 实现
Predefined Macros/Types:
  c_int / ONCE_FLAG_INIT (= 0)       // 类型与初始化器
  core::sync::atomic::AtomicI32      // 内部安全抽象的原子操作类型

[GUARANTEE]
Exported Interface:
  extern "C" fn call_once(flag: *mut core::ffi::c_int, func: Option<extern "C" fn()>);
                                      // 本模块保证对外提供与 C ABI 兼容的 call_once 符号
Internal Interface:
  pub(crate) fn __pthread_once(flag: *mut c_int, func: Option<extern "C" fn()>);
                                      // 内部一次性执行实现，供 call_once 委托调用
