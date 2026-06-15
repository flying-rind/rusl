# geteuid.c 规约

> musl libc POSIX 获取有效用户 ID 系统调用封装。`geteuid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
geteuid (Public)
  └── __syscall(SYS_geteuid) — 原始系统调用
```

---

## 函数规约

### geteuid

```c
uid_t geteuid(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取调用进程的有效用户 ID（Effective UID）。该调用是 `SYS_geteuid` 系统调用的零参数薄封装，总是成功。

有效用户 ID 决定进程执行大多数操作（如文件访问、信号发送）时的权限。对于未设置 SUID 位的程序，有效用户 ID 与真实用户 ID 相同。对于设置了 SUID 位的程序，有效用户 ID 为文件所有者的用户 ID。

#### 前置条件

- 无

#### 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的有效用户 ID

#### 系统算法

```
geteuid():
  return __syscall(SYS_geteuid)
```

即：
1. 调用 `__syscall(SYS_geteuid)` 执行内核系统调用
2. 内核返回当前进程的 `euid`（cred 结构体中的 `euid` 字段），直接返回给调用者

#### 依赖

- `SYS_geteuid` — Linux 内核系统调用编号 (x86_64: 107, aarch64: 175)
- `__syscall` — 内部宏，定义在 `src/internal/syscall.h`
