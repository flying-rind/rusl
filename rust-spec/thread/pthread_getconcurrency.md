# pthread_getconcurrency — Rust 接口归约

> rusl 获取建议并发级别（已废弃的 POSIX 函数）。在 Linux 上固定返回 0。

---

## 原始 C 对外导出接口

```c
// 用户可见符号
int pthread_getconcurrency(void);
```

---

## Rust 外部 ABI 接口

```rust
// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_getconcurrency() -> core::ffi::c_int;
```

---

## 意图

获取应用程序建议的线程并发级别。该函数在 POSIX.1-2001 和 POSIX.1-2008 中已被标记为废弃，musl 始终返回 0，表示由操作系统自行管理并发。

---

## 前置条件

- 无

---

## 后置条件

- 始终返回 0
- 无副作用：不修改任何全局或静态状态

---

## 不变量

无。该函数不访问任何全局状态。

---

## 算法

```
pthread_getconcurrency():
  1. return 0
```

Rust 内部实现极其简单：

```rust
pub extern "C" fn pthread_getconcurrency() -> core::ffi::c_int {
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::c_int                                        // 依赖1: C ABI 基本类型

Predefined Macros:
  (none)                                                   // 无任何依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_getconcurrency() -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_getconcurrency 符号
