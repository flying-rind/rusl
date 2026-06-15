# dup.c 规约

> musl libc POSIX 文件描述符复制系统调用封装。`dup` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
dup (Public)
  └── syscall(SYS_dup, fd)
        ├── __syscall(SYS_dup, fd) — 原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### dup

```c
int dup(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

复制文件描述符 `fd`，返回一个新的文件描述符，该新描述符：
- 引用相同的打开文件描述（file description）
- 共享文件偏移和状态标志
- 具有最低的可用编号
- 不共享 close-on-exec 标志（新 fd 的 FD_CLOEXEC 被清除）

#### 前置条件

- `fd`: 有效的已打开文件描述符
- 进程的文件描述符数量未达到 `RLIMIT_NOFILE` 上限

#### 后置条件

- **Case 1 成功**
  - 返回新的文件描述符（非负整数，不同于 `fd`）
  - 新 fd 与 `fd` 共享同一内核文件描述（共享文件偏移、状态标志）

- **Case 2 错误**
  - 返回 -1
  - `errno` 设置为 `EBADF`（fd 无效）或 `EMFILE`（达到进程 fd 上限）

#### 系统算法

```
dup(fd):
  return syscall(SYS_dup, fd)   // 直接封装内核 dup 系统调用
```

注意：使用 `syscall` 而非 `syscall_cp`，因为 dup 是快速操作，不需要取消点。

#### 依赖

- `SYS_dup` — Linux 内核系统调用编号
