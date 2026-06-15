# dup3.c 规约

> musl libc GNU 扩展文件描述符复制（指定编号 + 标志）系统调用封装。`dup3` 在 `<unistd.h>` 中（`_GNU_SOURCE`）声明。`__dup3` 是 musl 内部实现，`dup3` 是其弱别名。

---

## 依赖图

```
__dup3 (Internal, 导出为 dup3 的弱别名)
dup3 (Public, weak_alias of __dup3)
  ├── [SYS_dup2 存在时]
  │     ├── old==new: 返回 -EINVAL
  │     ├── flags!=0 且 SYS_dup3 可用:
  │     │     └── __syscall(SYS_dup3, old, new, flags) — 循环处理 EBUSY
  │     │           若 ENOSYS 且 flags 仅含 O_CLOEXEC:
  │     │             └── __syscall(SYS_dup2) + __syscall(SYS_fcntl, F_SETFD, FD_CLOEXEC)
  │     └── flags 无效: 返回 -EINVAL
  └── [SYS_dup2 不存在时]
        └── __syscall(SYS_dup3, old, new, flags) — 循环处理 EBUSY
```

---

## 函数规约

### __dup3 (dup3)

```c
int __dup3(int old, int new, int flags);
// weak_alias(__dup3, dup3);
int dup3(int old, int new, int flags);
```

[Visibility]:
- `__dup3`: Internal (不导出) — musl 内部实现，但通过弱别名暴露为 `dup3`
- `dup3`: User — GNU 扩展（`_GNU_SOURCE`），用户程序可调用

#### Intent

等价于 `dup2(old, new)`，但额外支持 `flags` 参数控制新文件描述符的行为。当前支持的 flag 为 `O_CLOEXEC`（设置 close-on-exec 标志）。与 dup2 不同，`dup3(old, new, flags)` 要求 `old != new`。

#### 前置条件

- `old`: 有效的已打开文件描述符
- `new`: `[0, OPEN_MAX)` 范围内的目标文件描述符编号
- `old != new`: 不允许 `old == new`（与 dup2 不同）
- `flags`: `O_CLOEXEC` 或其组合（Linux 5.3+ 无此限制）

#### 后置条件

- **Case 1 成功**
  - `new` 成为 `old` 的副本
  - 若 `flags & O_CLOEXEC`：`new` 设置 close-on-exec 标志
  - 返回 `new`（非负整数）

- **Case 2 `old == new`**
  - 返回 -1
  - `errno` 设置为 `EINVAL`

- **Case 3 `old` 无效或 `new` 超出范围**
  - 返回 -1
  - `errno` 设置为 `EBADF`

- **Case 4 `flags` 包含不支持的标志**
  - 返回 -1
  - `errno` 设置为 `EINVAL`

#### 系统算法

```
__dup3(old, new, flags):
  #ifdef SYS_dup2:
    if old == new:
      return __syscall_ret(-EINVAL)                       // posix 语义
    if flags:
      while ((r=__syscall(SYS_dup3, old, new, flags)) == -EBUSY)
        ;
      if r != -ENOSYS: return __syscall_ret(r)             // dup3 成功或不支持
      if flags & ~O_CLOEXEC: return __syscall_ret(-EINVAL) // 仅支持 O_CLOEXEC
    while ((r=__syscall(SYS_dup2, old, new)) == -EBUSY)    // 回退到 dup2
      ;
    if r >= 0 && (flags & O_CLOEXEC):
      __syscall(SYS_fcntl, new, F_SETFD, FD_CLOEXEC)       // 手动设置
  #else:
    while ((r=__syscall(SYS_dup3, old, new, flags)) == -EBUSY)
      ;
  return __syscall_ret(r)
```

#### 依赖

- `SYS_dup3` — Linux 内核系统调用
- `SYS_dup2` — 回退使用的旧版系统调用
- `SYS_fcntl` + `F_SETFD` + `FD_CLOEXEC` — 手动设置 close-on-exec
- `O_CLOEXEC` — GNU/POSIX close-on-exec 标志
