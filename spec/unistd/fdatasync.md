# fdatasync.c 规约

> musl libc POSIX 文件数据同步系统调用封装。`fdatasync` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
fdatasync (Public)
  └── syscall_cp(SYS_fdatasync, fd)
```

---

## 函数规约

### fdatasync

```c
int fdatasync(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将文件描述符 `fd` 的已修改数据同步到磁盘。与 `fsync` 不同，`fdatasync` 不强制刷新元数据（除非元数据对后续读取必要，如文件大小变化），因此可能比 `fsync` 更高效。

#### 前置条件

- `fd`: 有效的已打开文件描述符

#### 后置条件

- **Case 1 成功**: 所有缓冲数据已写入磁盘，返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
fdatasync(fd):
  return syscall_cp(SYS_fdatasync, fd)
```

#### 依赖

- `SYS_fdatasync` — Linux 内核系统调用
