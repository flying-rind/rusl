# fchownat.c 规约

> musl libc POSIX 相对路径改变文件所有者系统调用封装。`fchownat` 在 `<unistd.h>` 中声明。

---

## 函数规约

### fchownat

```c
int fchownat(int fd, const char *path, uid_t uid, gid_t gid, int flag);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

相对于目录文件描述符 `fd` 改变文件的所有者和/或所属组。支持 `AT_SYMLINK_NOFOLLOW` 标志（不跟随符号链接）。

#### 前置条件

- `fd`: 目录文件描述符或 `AT_FDCWD`
- `path`: 相对或绝对路径
- `uid` / `gid`: 新所有者/组（-1 保持不变）
- `flag`: 0 或 `AT_SYMLINK_NOFOLLOW`

#### 后置条件

- **Case 1 成功**: 返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
fchownat(fd, path, uid, gid, flag):
  return syscall(SYS_fchownat, fd, path, uid, gid, flag)
```

纯系统调用封装。

#### 依赖

- `SYS_fchownat` — Linux 内核系统调用
