# readlinkat.c 规约

> musl libc POSIX 相对路径读取符号链接目标系统调用封装。`readlinkat` 在 `<unistd.h>` 中声明。

---

## 函数规约

### readlinkat

```c
ssize_t readlinkat(int fd, const char *restrict path, char *restrict buf, size_t bufsize);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

相对于目录文件描述符 `fd` 读取符号链接的目标路径。与 `readlink` 相同的语义，但支持相对路径。

#### 前置条件

- `fd`: 目录文件描述符或 `AT_FDCWD`
- `path`: 相对或绝对符号链接路径
- `buf` / `bufsize`: 同 readlink

#### 后置条件

- **Case 1 成功**: 返回写入字节数（不含 '\0'），bufsize==0 时返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
readlinkat(fd, path, buf, bufsize):
  if !bufsize: buf = dummy; bufsize = 1   // 同 readlink 的 bufsize==0 保护
  r = __syscall(SYS_readlinkat, fd, path, buf, bufsize)
  if buf == dummy && r > 0: r = 0
  return __syscall_ret(r)
```

#### 依赖

- `SYS_readlinkat` — Linux 内核系统调用
