# chdir.c 规约

> musl libc POSIX 改变工作目录系统调用封装。`chdir` 在 `<unistd.h>` 中声明。

---

## 函数规约

### chdir

```c
int chdir(const char *path);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将调用进程的当前工作目录改为 `path` 指定的目录。

#### 前置条件

- `path`: 以 NULL 结尾的有效目录路径字符串

#### 后置条件

- **Case 1 成功**: 当前工作目录变更为 `path`，返回 0
- **Case 2 错误**: 返回 -1，设置 errno（`ENOENT`、`EACCES`、`ENOTDIR`、`ELOOP` 等）

#### 系统算法

```
chdir(path):
  return syscall(SYS_chdir, path)
```

#### 依赖

- `SYS_chdir` — Linux 内核系统调用
