# access.c 规约

> musl libc POSIX 文件访问权限检查系统调用封装。`access` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
access (Public)
  ├── [SYS_access 存在时] syscall(SYS_access, filename, amode)
  └── [SYS_access 不存在时] syscall(SYS_faccessat, AT_FDCWD, filename, amode, 0)
```

---

## 函数规约

### access

```c
int access(const char *filename, int amode);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

使用调用进程的**真实**（而非有效）UID/GID 检查对文件 `filename` 的访问权限。与直接尝试 `open()` 不同，access 不会因有效 UID 的特权而错误报告权限。

#### 前置条件

- `filename`: 以 NULL 结尾的有效文件路径字符串
- `amode`: `F_OK` (0) 测试存在性，或 `R_OK|W_OK|X_OK` 的按位或组合

#### 后置条件

- **Case 1 允许访问**: 返回 0
- **Case 2 拒绝访问或文件不存在**: 返回 -1，errno = `EACCES`（权限不足）或 `ENOENT`（文件不存在）
- **Case 3 其他错误**: 返回 -1，设置 errno（`ENAMETOOLONG`、`ENOTDIR`、`ELOOP`、`EIO` 等）

#### 系统算法

```
access(filename, amode):
  #ifdef SYS_access:
    return syscall(SYS_access, filename, amode)
  #else:
    return syscall(SYS_faccessat, AT_FDCWD, filename, amode, 0)
  #endif
```

#### 依赖

- `SYS_access` — Linux 内核系统调用（部分架构）
- `SYS_faccessat` + `AT_FDCWD` — 回退方案
