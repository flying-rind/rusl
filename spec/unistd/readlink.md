# readlink.c 规约

> musl libc POSIX 读取符号链接目标系统调用封装。`readlink` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
readlink (Public)
  ├── bufsize==0: 使用栈上 dummy[1] 防止内核写空指针
  │     └── __syscall(SYS_readlink / SYS_readlinkat, path, buf, bufsize)
  │           └── buf == dummy && r > 0: r = 0  // 恢复为返回 0
  └── __syscall_ret(r)
```

---

## 函数规约

### readlink

```c
ssize_t readlink(const char *restrict path, char *restrict buf, size_t bufsize);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

读取符号链接 `path` 的目标路径，将最多 `bufsize` 字节（不含终止 '\0'）复制到 `buf` 中。注意返回的路径字符串**不**以 '\0' 结尾。

#### 前置条件

- `path`: 已存在的符号链接路径
- `buf`: 指向至少 `bufsize` 字节的缓冲区（若 bufsize>0）
- `bufsize`: 缓冲区大小（0 表示只验证符号链接存在而不读取内容）

#### 后置条件

- **Case 1 成功**
  - 返回写入 `buf` 的字节数（不含 '\0'）
  - `buf` 包含目标路径的内容前缀（若 bufsize 小于链接目标长度）
  - 若 bufsize==0：返回 0（musl 特殊处理，防止内核写 NULL）

- **Case 2 错误**
  - 返回 -1
  - `errno` 设置为 `ENOENT`（不是符号链接）、`EINVAL`（不是符号链接）等

#### 系统算法

```
readlink(path, buf, bufsize):
  if !bufsize:
    buf = dummy; bufsize = 1                             // 防止内核空指针解引用

  r = __syscall(SYS_readlink / SYS_readlinkat(AT_FDCWD), path, buf, bufsize)

  if buf == dummy && r > 0: r = 0                        // bufsize==0 的情况恢复

  return __syscall_ret(r)
```

#### 不变量

- 当 `bufsize == 0` 时，函数使用私有 `dummy[1]` 栈缓冲区保护内核调用，然后返回 0

#### 依赖

- `SYS_readlink` / `SYS_readlinkat` — Linux 内核系统调用
