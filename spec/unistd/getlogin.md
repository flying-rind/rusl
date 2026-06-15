# getlogin.c 规约

> musl libc POSIX 登录用户名获取函数。`getlogin` 在 `<unistd.h>` 中声明。非线程安全版本，返回指向环境变量中 `LOGNAME` 值的指针。

---

## 依赖图

```
getlogin (Public)
  └── getenv("LOGNAME") — 获取环境变量 LOGNAME 的值
```

---

## 函数规约

### getlogin

```c
char *getlogin(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

返回当前登录用户的名字。musl 的实现直接查询环境变量 `LOGNAME`，这是 POSIX 允许的简单实现方式。环境变量 `LOGNAME` 通常由登录程序（如 `login`、`sshd`）设置。

#### 前置条件

- 进程环境变量中存在 `LOGNAME`

#### 后置条件

- **Case 1 `LOGNAME` 环境变量存在**
  - 返回指向 `LOGNAME` 环境变量值的指针
  - 返回的指针指向进程环境变量表，后续 `setenv`/`putenv` 可能使其失效

- **Case 2 `LOGNAME` 环境变量不存在**
  - 返回 `NULL`
  - `errno` 可能未设置

#### 系统算法

```
getlogin():
  return getenv("LOGNAME")          // 1. 查询 LOGNAME 环境变量
```

即：
1. 调用 `getenv("LOGNAME")` 获取环境变量值
2. 直接返回其结果：找到则返回指针，未找到则返回 `NULL`

#### 依赖

- `getenv` — 获取环境变量值（`src/env/getenv.c`）

#### 安全说明

此实现依赖环境变量 `LOGNAME`，该变量可能被进程自身或父进程修改。对于安全敏感的场景（如审计日志），应使用 `getlogin_r` 并通过 `/var/run/utmp` 或其他系统机制验证。
