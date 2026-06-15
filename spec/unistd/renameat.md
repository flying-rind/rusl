# renameat.c 规约

> musl libc POSIX 相对路径重命名文件系统调用封装。`renameat` 在 `<stdio.h>` 中声明。

---

## 函数规约

### renameat

```c
int renameat(int oldfd, const char *old, int newfd, const char *new);
```

[Visibility]: User — `<stdio.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

相对于目录文件描述符原子性地重命名文件/目录。

#### 前置条件

- `oldfd`: `old` 的基目录 fd 或 `AT_FDCWD`
- `newfd`: `new` 的基目录 fd 或 `AT_FDCWD`
- `old` / `new`: 相对或绝对路径

#### 后置条件

- **Case 1 成功**: 文件被重命名，返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
renameat(oldfd, old, newfd, new):
  #ifdef SYS_renameat:
    return syscall(SYS_renameat, oldfd, old, newfd, new)
  #else:
    return syscall(SYS_renameat2, oldfd, old, newfd, new, 0)
  #endif
```

#### 依赖

- `SYS_renameat` / `SYS_renameat2` — Linux 内核系统调用
