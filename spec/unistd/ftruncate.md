# ftruncate.c 规约

> musl libc POSIX 文件截断系统调用封装。`ftruncate` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
ftruncate (Public)
  └── syscall(SYS_ftruncate, fd, __SYSCALL_LL_O(length))
```

---

## 函数规约

### ftruncate

```c
int ftruncate(int fd, off_t length);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将文件描述符 `fd` 引用的普通文件截断为精确 `length` 字节。若文件之前大于 `length`，超出部分被丢弃；若文件之前小于 `length`，扩展部分填充为零（形成稀疏文件）。

#### 前置条件

- `fd`: 有效且以写模式打开的文件描述符
- `length`: 非负的文件大小（字节）

#### 后置条件

- **Case 1 成功**: 文件大小变为 `length`，返回 0
- **Case 2 错误**: 返回 -1，设置 errno（`EBADF`、`EINVAL`、`EFBIG` 等）

#### 系统算法

```
ftruncate(fd, length):
  return syscall(SYS_ftruncate, fd, __SYSCALL_LL_O(length))
```

`__SYSCALL_LL_O(length)` 在 32 位平台上将 `off_t`（64 位）展开为两个 32 位参数（低 32 位在前）。

#### 依赖

- `SYS_ftruncate` — Linux 内核系统调用
- `__SYSCALL_LL_O` — 32/64 位长度适配宏
