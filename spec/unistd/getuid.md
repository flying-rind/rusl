# getuid.c 规约

> musl libc POSIX 获取真实用户 ID 系统调用封装。`getuid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getuid (Public)
  └── __syscall(SYS_getuid) — 原始系统调用
```

---

## 函数规约

### getuid

```c
uid_t getuid(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取调用进程的真实用户 ID（Real UID）。该调用是 `SYS_getuid` 系统调用的零参数薄封装，总是成功。

#### 前置条件

- 无

#### 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的真实用户 ID
  - 该值在进程生命周期内通常不变（除非通过 `setuid`/`setreuid`/`setresuid` 修改）

#### 系统算法

```
getuid():
  return __syscall(SYS_getuid)
```

即：
1. 调用 `__syscall(SYS_getuid)` 执行内核系统调用
2. 内核返回当前进程的 `uid`（cred 结构体中的 `uid` 字段），直接返回给调用者

#### 依赖

- `SYS_getuid` — Linux 内核系统调用编号 (x86_64: 102, aarch64: 174)
- `__syscall` — 内部宏，定义在 `src/internal/syscall.h`
