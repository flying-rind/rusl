# getgid.c 规约

> musl libc POSIX 获取真实组 ID 系统调用封装。`getgid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getgid (Public)
  └── __syscall(SYS_getgid) — 原始系统调用
```

---

## 函数规约

### getgid

```c
gid_t getgid(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取调用进程的真实组 ID（Real GID）。该调用是 `SYS_getgid` 系统调用的零参数薄封装，总是成功。

#### 前置条件

- 无

#### 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的真实组 ID
  - 该值在进程生命周期内通常不变（除非通过 `setgid`/`setregid`/`setresgid` 修改）

#### 系统算法

```
getgid():
  return __syscall(SYS_getgid)
```

即：
1. 调用 `__syscall(SYS_getgid)` 执行内核系统调用
2. 内核返回当前进程的 `gid`（cred 结构体中的 `gid` 字段），直接返回给调用者

#### 依赖

- `SYS_getgid` — Linux 内核系统调用编号 (x86_64: 104, aarch64: 176)
- `__syscall` — 内部宏，定义在 `src/internal/syscall.h`
