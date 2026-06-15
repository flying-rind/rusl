# symlink.c 规约

> musl libc POSIX 创建符号链接系统调用封装。`symlink` 在 `<unistd.h>` 中声明。

---

## 函数规约

### symlink

```c
int symlink(const char *existing, const char *new);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

创建符号链接 `new` 指向目标 `existing`（目标不需要存在）。符号链接是包含目标路径字符串的特殊文件类型。

#### 前置条件

- `existing`: 链接目标路径字符串（可为任意字符串，甚至不存在的路径）
- `new`: 符号链接的路径名（必须不存在）

#### 后置条件

- **Case 1 成功**: 创建符号链接，返回 0
- **Case 2 错误**: 返回 -1，设置 errno（`EEXIST`、`ENOENT`、`EACCES` 等）

#### 系统算法

```
symlink(existing, new):
  #ifdef SYS_symlink:
    return syscall(SYS_symlink, existing, new)
  #else:
    return syscall(SYS_symlinkat, existing, AT_FDCWD, new)
  #endif
```

#### 依赖

- `SYS_symlink` / `SYS_symlinkat` — Linux 内核系统调用
