# unlink.c 规约

> musl libc POSIX 删除文件名系统调用封装。`unlink` 在 `<unistd.h>` 中声明。

---

## 函数规约

### unlink

```c
int unlink(const char *path);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

删除文件系统中 `path` 指定的文件名（目录条目）。若该文件名是文件的最后一个硬链接且没有进程打开该文件，文件数据被删除。若 `path` 是符号链接，删除链接自身而非目标。

不能用于删除目录（使用 `rmdir` 或 `unlinkat(..., AT_REMOVEDIR)`）。

#### 前置条件

- `path`: 以 NULL 结尾的有效文件路径（不能是目录）

#### 后置条件

- **Case 1 成功**: 文件名被删除，返回 0
- **Case 2 错误**: 返回 -1，errno = `EPERM`（目录）、`ENOENT`、`EACCES`、`EBUSY` 等

#### 系统算法

```
unlink(path):
  #ifdef SYS_unlink:
    return syscall(SYS_unlink, path)
  #else:
    return syscall(SYS_unlinkat, AT_FDCWD, path, 0)
  #endif
```

#### 依赖

- `SYS_unlink` / `SYS_unlinkat` — Linux 内核系统调用
