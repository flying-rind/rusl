# rmdir.c 规约

> musl libc POSIX 删除空目录系统调用封装。`rmdir` 在 `<unistd.h>` 中声明。

---

## 函数规约

### rmdir

```c
int rmdir(const char *path);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

删除空目录 `path`。若目录非空（包含 `.` 和 `..` 以外的条目），删除失败。

#### 前置条件

- `path`: 存在的空目录路径
- 调用者对目录有写权限

#### 后置条件

- **Case 1 成功**: 目录被删除，返回 0
- **Case 2 目录非空**: 返回 -1，errno = `ENOTEMPTY`
- **Case 3 其他错误**: 返回 -1，设置 errno

#### 系统算法

```
rmdir(path):
  #ifdef SYS_rmdir:
    return syscall(SYS_rmdir, path)
  #else:
    return syscall(SYS_unlinkat, AT_FDCWD, path, AT_REMOVEDIR)
  #endif
```

#### 依赖

- `SYS_rmdir` / `SYS_unlinkat` + `AT_REMOVEDIR` — Linux 内核系统调用
