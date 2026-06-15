# linkat.c 规约

> musl libc POSIX 相对路径创建硬链接系统调用封装。`linkat` 在 `<unistd.h>` 中声明。

---

## 函数规约

### linkat

```c
int linkat(int fd1, const char *existing, int fd2, const char *new, int flag);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

相对于目录文件描述符创建硬链接。支持 `AT_SYMLINK_FOLLOW` 标志。

#### 前置条件

- `fd1`: `existing` 的基目录 fd 或 `AT_FDCWD`
- `fd2`: `new` 的基目录 fd 或 `AT_FDCWD`
- `existing` / `new`: 相对或绝对路径
- `flag`: 0 或 `AT_SYMLINK_FOLLOW`

#### 后置条件

- **Case 1 成功**: 返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
linkat(fd1, existing, fd2, new, flag):
  return syscall(SYS_linkat, fd1, existing, fd2, new, flag)
```

#### 依赖

- `SYS_linkat` — Linux 内核系统调用
