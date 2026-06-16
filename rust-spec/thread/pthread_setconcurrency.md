# pthread_setconcurrency — Rust 接口归约

> rusl 设置建议并发级别（已废弃的 POSIX 函数）。参数校验后，非零值均返回失败。

---

## 原始 C 对外导出接口

```c
// 用户可见符号
int pthread_setconcurrency(int val);
```

---

## Rust 外部 ABI 接口

```rust
// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_setconcurrency(val: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图

设置应用程序建议的线程并发级别。该函数在 POSIX.1-2001 和 POSIX.1-2008 中已被标记为废弃。musl 的实现仅做参数合法性校验：`val < 0` 返回 `EINVAL`，`val > 0` 返回 `EAGAIN`，`val == 0` 返回 0。

---

## 前置条件

- 无

---

## 后置条件

- Case 1 (`val < 0`): 返回 `EINVAL`，表示参数非法
- Case 2 (`val > 0`): 返回 `EAGAIN`，表示指定的并发级别当前无法满足
- Case 3 (`val == 0`): 返回 0，成功（操作系统自行管理并发）
- 无副作用：不修改任何全局或静态状态

---

## 不变量

无。该函数不访问任何全局状态。

---

## 算法

```
pthread_setconcurrency(val):
  1. if val < 0:  return EINVAL
  2. if val > 0:  return EAGAIN
  3. return 0
```

Rust 内部实现：

```rust
pub extern "C" fn pthread_setconcurrency(val: core::ffi::c_int) -> core::ffi::c_int {
    if val < 0 {
        EINVAL
    } else if val > 0 {
        EAGAIN
    } else {
        0
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::c_int                                        // 依赖1: C ABI 基本类型

Predefined Macros/Constants:
  EINVAL, EAGAIN                                           // 错误码宏定义

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_setconcurrency(val: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_setconcurrency 符号
