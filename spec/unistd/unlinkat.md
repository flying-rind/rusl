# unlinkat.c 规约

> musl libc POSIX 相对路径删除文件名系统调用封装。`unlinkat` 在 `<unistd.h>` 中声明。

---

## 函数规约

### unlinkat

```c
int unlinkat(int fd, const char *path, int flag);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

相对于目录文件描述符 `fd` 删除文件名。支持 `AT_REMOVEDIR` 标志（删除目录而非文件）。

#### 前置条件

- `fd`: 目录 fd 或 `AT_FDCWD`
- `path`: 相对或绝对路径
- `flag`: 0 或 `AT_REMOVEDIR`

#### 后置条件

- **Case 1 成功**: 返回 0
- **Case 2 错误**: 返回 -1

#### 系统算法

```
unlinkat(fd, path, flag):
  return syscall(SYS_unlinkat, fd, path, flag)
```

#### 依赖

- `SYS_unlinkat` — Linux 内核系统调用
