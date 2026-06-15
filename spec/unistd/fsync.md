# fsync.c 规约

> musl libc POSIX 文件同步系统调用封装。`fsync` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
fsync (Public)
  └── syscall_cp(SYS_fsync, fd)
```

---

## 函数规约

### fsync

```c
int fsync(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将文件描述符 `fd` 的所有已修改数据和元数据（文件大小、访问时间等）同步到磁盘。确保系统崩溃后数据不会丢失。

#### 前置条件

- `fd`: 有效的已打开文件描述符（通常需要写权限）

#### 后置条件

- **Case 1 成功**: 所有缓冲数据已写入磁盘，返回 0
- **Case 2 错误**: 返回 -1，设置 errno（`EBADF`、`EIO`、`EROFS` 等）

#### 系统算法

```
fsync(fd):
  return syscall_cp(SYS_fsync, fd)
```

#### 依赖

- `SYS_fsync` — Linux 内核系统调用
