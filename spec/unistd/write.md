# write.c 规约

> musl libc POSIX 文件写入系统调用封装。`write` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
write (Public)
  └── syscall_cp(SYS_write, fd, buf, count)
        ├── __syscall_cp(...) — 可被信号取消的原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### write

```c
ssize_t write(int fd, const void *buf, size_t count);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将缓冲区 `buf` 中至多 `count` 字节写入文件描述符 `fd`。该调用是 `SYS_write` 系统调用的薄封装，使用 `syscall_cp` 确保线程取消安全。

#### 前置条件

- `fd`: 已打开的、以写模式打开的有效文件描述符
- `buf`: 指向至少 `count` 字节数据的非空指针
- `count`: `[0, SSIZE_MAX]` 范围内的字节数

#### 后置条件

- **Case 1 成功写入**
  - 返回实际写入的字节数（可能小于 `count`，如磁盘满或管道缓冲区满）
  - 如果是普通文件，文件位置前进相应字节数
  - `buf` 中前 `r` 字节已提交到 `fd`

- **Case 2 调用被信号中断**
  - 返回 -1
  - `errno` 设置为 `EINTR`

- **Case 3 其他错误**
  - 返回 -1
  - `errno` 设置为对应错误码（如 `EBADF`、`EFAULT`、`EIO`、`EPIPE` 等）

#### 系统算法

```
write(fd, buf, count):
  return syscall_cp(SYS_write, fd, buf, count)
```

即：
1. 调用 `__syscall_cp(SYS_write, fd, buf, count)` 执行内核系统调用（线程取消点）
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定

#### 依赖

- `SYS_write` — Linux 内核系统调用编号 (x86_64: 1, aarch64: 64)
- `syscall_cp` — 内部宏，定义在 `src/internal/syscall.h`
