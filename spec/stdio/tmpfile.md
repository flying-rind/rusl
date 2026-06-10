# tmpfile.c 规约

> musl libc 标准库临时文件创建函数实现。创建一个临时文件，当文件关闭或程序退出时自动删除。

---

## 依赖图

```
tmpfile (Public)
  ├── __randname(s+13)                    — 生成随机文件名后缀 (src/temp/__randname.c)
  ├── sys_open(s, O_RDWR|O_CREAT|O_EXCL, 0600) — 创建并打开文件 (syscall.h)
  ├── __syscall(SYS_unlink, s)            — 立即删除目录项 (syscall.h, 条件编译)
  ├── __syscall(SYS_unlinkat, ...)        — 相对路径删除 (fallback, syscall.h)
  ├── __fdopen(fd, "w+")                 — 从 fd 创建 FILE (src/stdio/__fdopen.c)
  └── __syscall(SYS_close, fd)           — 清理：关闭 fd (syscall.h)
```

---

## 函数规约

### 1. tmpfile

```c
FILE *tmpfile(void);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

在 `/tmp` 目录下创建一个临时文件，以 `"w+"`（读写）模式打开，并在创建后立即执行 `unlink` 操作。该文件在 `FILE` 关闭或程序退出时自动被系统回收（因目录项已删除，仅剩引用计数）。提供安全的临时文件创建（使用 `O_EXCL` 防止文件名冲突），最多重试 100 次。

#### 前置条件

- `/tmp` 目录存在且可写
- 调用进程对 `/tmp` 拥有写权限和搜索权限
- 系统有足够的 inode 和磁盘空间

#### 后置条件

- **Case 1 成功创建（100 次尝试内）**
  - 在 `/tmp` 下创建唯一命名的文件，以 `0600` 权限打开
  - 文件的目录项立即被 `unlink`（仅通过 fd 访问）
  - 返回读写双模式 `FILE*`（`w+`）
  - 调用者负责 `fclose`，关闭时文件数据自动释放

- **Case 2 所有尝试失败（100 次内无唯一文件名或系统错误）**
  - 返回 `NULL`
  - `errno` 设置为最后一个系统调用的错误码

#### 系统算法

```
tmpfile():
  s[22] = "/tmp/tmpfile_XXXXXX"    // 固定前缀 + 6位随机占位符
  
  for try in [0, MAXTRIES=100):
    1. __randname(s+13)             // 将 s[13..18] 替换为随机字母数字
    2. fd = sys_open(s, O_RDWR|O_CREAT|O_EXCL, 0600)  // 原子创建+打开
    3. if fd >= 0:                  // 创建成功
         a. unlink(s)               // 立即删除目录项（SYS_unlink 或 SYS_unlinkat）
         b. f = __fdopen(fd, "w+")  // 从 fd 创建 FILE 流
         c. if !f:                  // __fdopen 失败
              __syscall(SYS_close, fd)  // 手动关闭 fd
         d. return f                // 返回 FILE*(可能为 NULL)
  循环结束
  return 0  // 所有尝试失败
```

#### 不变量

- 创建的文件权限始终为 `0600`（仅 owner 可读写）
- 文件一旦 `unlink` 后即不可通过路径访问，仅通过返回的 `FILE*` 操作
- 最多 `MAXTRIES=100` 次尝试

#### 依赖

- `__randname(char *template)` — 将模板尾部 6 个 `X` 替换为随机字母数字字符（定义于 `src/temp/__randname.c`）
- `sys_open(...)` — 打开/创建文件系统调用宏（定义于 `src/internal/syscall.h`）
- `__syscall(SYS_unlink, ...)` / `__syscall(SYS_unlinkat, ...)` — 删除目录项系统调用（定义于 `src/internal/syscall.h`）
- `__fdopen(int fd, const char *mode)` — 从文件描述符创建 FILE（定义于 `src/stdio/__fdopen.c`）
- `__syscall(SYS_close, fd)` — 关闭文件描述符（定义于 `src/internal/syscall.h`）
- `O_RDWR / O_CREAT / O_EXCL` — open 标志（来自 `<fcntl.h>`）
- `AT_FDCWD` — 当前工作目录常量（来自 `<fcntl.h>`，条件编译分支）

#### 安全特性

- `O_CREAT | O_EXCL` 保证原子创建，如果文件名已存在则 `sys_open` 失败
- 创建后立即 `unlink`，确保即使进程异常退出文件也会被系统清理（目录项已不存在，inode 引用计数归零后系统自动回收）
