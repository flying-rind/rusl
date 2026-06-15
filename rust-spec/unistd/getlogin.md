# getlogin — Rust 接口归约

## 原始 C 接口
```c
char *getlogin(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn getlogin() -> *mut core::ffi::c_char;
```

---

## 意图

返回当前登录用户的名字。musl 的实现直接查询环境变量 `LOGNAME`，这是 POSIX 允许的简单实现方式。环境变量 `LOGNAME` 通常由登录程序（如 `login`、`sshd`）设置。

## 前置条件

- 进程环境变量中存在 `LOGNAME`

## 后置条件

- **Case 1 `LOGNAME` 环境变量存在**
  - 返回指向 `LOGNAME` 环境变量值的指针
  - 返回的指针指向进程环境变量表，后续 `setenv`/`putenv` 可能使其失效

- **Case 2 `LOGNAME` 环境变量不存在**
  - 返回 `NULL`
  - `errno` 可能未设置

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现直接调用 `getenv("LOGNAME")`。Rust 中：

```rust
extern "C" fn getlogin() -> *mut core::ffi::c_char {
    getenv(c"LOGNAME".as_ptr())
}
```

即：
1. 调用 `getenv("LOGNAME")` 获取环境变量值
2. 直接返回其结果：找到则返回指针，未找到则返回 `NULL`

---

## 安全说明

此实现依赖环境变量 `LOGNAME`，该变量可能被进程自身或父进程修改。对于安全敏感的场景（如审计日志），应使用 `getlogin_r` 并通过 `/var/run/utmp` 或其他系统机制验证。

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 Option<&str>
/// 注意：返回的字符串引用了进程环境变量表
pub(crate) fn login_name() -> Option<&'static str> {
    // 查询 LOGNAME 环境变量
    get_env_var("LOGNAME")
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  getenv — 获取环境变量值
  LOGNAME — 标准环境变量名（值为 "LOGNAME"）
Predefined Macros/Crates:
  env 模块 (rusl 内部环境变量管理)

[GUARANTEE]
Exported Interface:
  extern "C" fn getlogin() -> *mut core::ffi::c_char;
                                 // 本模块保证对外提供与 C ABI 兼容的 getlogin 符号
Internal Interface:
  pub(crate) fn login_name() -> Option<&'static str>;
                                 // 安全 Rust 包装
