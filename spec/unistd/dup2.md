# dup2.c 规约

> musl libc POSIX 文件描述符复制（指定编号）系统调用封装。`dup2` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
dup2 (Public)
  ├── [SYS_dup2 存在时]
  │     └── __syscall(SYS_dup2, old, new) — 循环处理 EBUSY
  │           └── __syscall_ret(...)
  └── [SYS_dup2 不存在时]
        ├── old==new: __syscall(SYS_fcntl, old, F_GETFD) — 检查 fd 有效性
        └── old!=new: __syscall(SYS_dup3, old, new, 0) — 循环处理 EBUSY
              └── __syscall_ret(...)
```

---

## 函数规约

### dup2

```c
int dup2(int old, int new);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

复制文件描述符 `old`，使用 `new` 作为新文件描述符编号。如果 `new` 已打开，先关闭它。两个描述符共享同一内核文件描述。使用原子性的 dup2/dup3 系统调用避免竞态条件。

#### 前置条件

- `old`: 有效的已打开文件描述符
- `new`: `[0, OPEN_MAX)` 范围内的目标文件描述符编号
- `old != new`: 允许 old==new（dup2 规范要求成功返回 new）

#### 后置条件

- **Case 1 成功**
  - `new` 成为 `old` 的副本
  - `new` 的 FD_CLOEXEC 标志被清除
  - 返回 `new`（非负整数）

- **Case 2 `old` 无效**
  - 返回 -1
  - `errno` 设置为 `EBADF`

- **Case 3 `new` 超出范围**
  - 返回 -1
  - `errno` 设置为 `EBADF`

#### 系统算法

```
dup2(old, new):
  #ifdef SYS_dup2:
    while (__syscall(SYS_dup2, old, new) == -EBUSY)  // 原子操作，处理 kernel EBUSY
      ;
    return __syscall_ret(r)
  #else:
    if old == new:
      r = __syscall(SYS_fcntl, old, F_GETFD)          // 仅检查 old 有效性
      if r >= 0: return old
    else:
      while (__syscall(SYS_dup3, old, new, 0) == -EBUSY) // 使用 dup3(old, new, 0)
        ;
    return __syscall_ret(r)
```

#### 依赖

- `SYS_dup2` — Linux 内核系统调用（旧版）
- `SYS_dup3` — Linux 内核 dup3 系统调用（flags=0 回退）
- `SYS_fcntl` + `F_GETFD` — 检查文件描述符有效性
