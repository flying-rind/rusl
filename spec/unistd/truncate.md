# truncate.c 规约

> musl libc POSIX 截断文件系统调用封装。`truncate` 在 `<unistd.h>` 中声明。

---

## 函数规约

### truncate

```c
int truncate(const char *path, off_t length);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将 `path` 指定的普通文件截断为精确 `length` 字节。与 `ftruncate` 类似，但通过路径名操作。

#### 前置条件

- `path`: 已存在普通文件的路径
- `length`: 非负整数

#### 后置条件

- **Case 1 成功**: 文件大小变为 `length`，返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
truncate(path, length):
  return syscall(SYS_truncate, path, __SYSCALL_LL_O(length))
```

#### 依赖

- `SYS_truncate` — Linux 内核系统调用
- `__SYSCALL_LL_O` — 32/64 位长度适配
