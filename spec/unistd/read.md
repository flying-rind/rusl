# read.c 规约

> musl libc POSIX 文件读取系统调用封装。`read` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
read (Public)
  └── syscall_cp(SYS_read, fd, buf, count)
        ├── __syscall_cp(...) — 可被信号取消的原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### read

```c
ssize_t read(int fd, void *buf, size_t count);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

从文件描述符 `fd` 中读取至多 `count` 字节到缓冲区 `buf` 中。该调用是 `SYS_read` 系统调用的薄封装，使用 `syscall_cp` 确保可以被信号安全取消。

#### 前置条件

- `fd`: 已打开的、以读模式（或读/写模式）打开的有效文件描述符
- `buf`: 指向至少 `count` 字节可用内存的非空指针
- `count`: `[0, SSIZE_MAX]` 范围内的字节数

#### 后置条件

- **Case 1 成功读取**
  - 返回实际读取的字节数（可能小于 `count`）
  - 如果是普通文件，文件位置前进相应字节数
  - 返回 0 表示到达文件末尾（EOF）

- **Case 2 调用被信号中断且未读取任何数据**
  - 返回 -1
  - `errno` 设置为 `EINTR`

- **Case 3 其他错误**
  - 返回 -1
  - `errno` 设置为对应错误码（如 `EBADF`、`EFAULT`、`EIO` 等）

#### 系统算法

```
read(fd, buf, count):
  return syscall_cp(SYS_read, fd, buf, count)
```

即：
1. 调用 `__syscall_cp(SYS_read, fd, buf, count)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

#### 依赖

- `SYS_read` — Linux 内核系统调用编号 (x86_64: 0, aarch64: 63)
- `syscall_cp` — 内部宏，定义在 `src/internal/syscall.h`
