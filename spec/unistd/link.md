# link.c 规约

> musl libc POSIX 创建硬链接系统调用封装。`link` 在 `<unistd.h>` 中声明。

---

## 函数规约

### link

```c
int link(const char *existing, const char *new);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

创建文件 `existing` 的新硬链接 `new`。两个路径名指向同一 inode，共享所有数据和元数据，删除任一链接不会影响另一个。

#### 前置条件

- `existing`: 已存在文件（不能是目录，除非是 root）
- `new`: 必须不存在（不能覆盖已有文件）
- `existing` 和 `new` 不能在不同文件系统上

#### 后置条件

- **Case 1 成功**: 创建硬链接，返回 0
- **Case 2 错误**: 返回 -1，设置 errno（`EEXIST`、`EXDEV`、`EPERM`、`ENOENT` 等）

#### 系统算法

```
link(existing, new):
  #ifdef SYS_link:
    return syscall(SYS_link, existing, new)
  #else:
    return syscall(SYS_linkat, AT_FDCWD, existing, AT_FDCWD, new, 0)
  #endif
```

#### 依赖

- `SYS_link` / `SYS_linkat` — Linux 内核系统调用
