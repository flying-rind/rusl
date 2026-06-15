# getegid.c 规约

> musl libc POSIX 获取有效组 ID 系统调用封装。`getegid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getegid (Public)
  └── __syscall(SYS_getegid) — 原始系统调用
```

---

## 函数规约

### getegid

```c
gid_t getegid(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取调用进程的有效组 ID（Effective GID）。该调用是 `SYS_getegid` 系统调用的零参数薄封装，总是成功。

有效组 ID 决定进程执行文件访问等操作时的组权限。对于未设置 SGID 位的程序，有效组 ID 与真实组 ID 相同。对于设置了 SGID 位的程序，有效组 ID 为文件所属组的组 ID。

#### 前置条件

- 无

#### 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的有效组 ID

#### 系统算法

```
getegid():
  return __syscall(SYS_getegid)
```

即：
1. 调用 `__syscall(SYS_getegid)` 执行内核系统调用
2. 内核返回当前进程的 `egid`（cred 结构体中的 `egid` 字段），直接返回给调用者

#### 依赖

- `SYS_getegid` — Linux 内核系统调用编号 (x86_64: 108, aarch64: 177)
- `__syscall` — 内部宏，定义在 `src/internal/syscall.h`
