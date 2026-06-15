# symlinkat.c 规约

> musl libc POSIX 相对路径创建符号链接系统调用封装。`symlinkat` 在 `<unistd.h>` 中声明。

---

## 函数规约

### symlinkat

```c
int symlinkat(const char *existing, int fd, const char *new);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

相对于目录文件描述符 `fd` 创建符号链接。

#### 前置条件

- `existing`: 链接目标路径字符串
- `fd`: 新链接的基目录 fd 或 `AT_FDCWD`
- `new`: 相对或绝对路径名

#### 后置条件

- **Case 1 成功**: 返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
symlinkat(existing, fd, new):
  return syscall(SYS_symlinkat, existing, fd, new)
```

#### 依赖

- `SYS_symlinkat` — Linux 内核系统调用
