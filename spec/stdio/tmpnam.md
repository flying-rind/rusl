# tmpnam.c 规约

> musl libc 标准库临时文件名生成函数实现。生成一个唯一的临时文件路径名。

---

## 依赖图

```
tmpnam (Public)
  ├── __randname(s+12)                              — 生成随机文件名后缀 (src/temp/__randname.c)
  ├── __syscall(SYS_readlink, ...)                  — 检测文件存在 (syscall.h)
  ├── __syscall(SYS_readlinkat, AT_FDCWD, ...)      — 相对路径检测 (fallback, syscall.h)
  └── strcpy(buf ? buf : internal, s)               — 复制路径到目标缓冲区 (<string.h>)
```

---

## 函数规约

### 1. tmpnam

```c
char *tmpnam(char *buf);
```

[Visibility]: User — `<stdio.h>` 标准库函数（C89，已被标记为过时），用户程序可直接调用

#### Intent

生成一个不与现有文件冲突的临时文件名。通过生成 `/tmp/tmpnam_XXXXXX` 形式的路径名（将 `XXXXXX` 替换为随机字符），然后使用 `readlink` 系统调用来检测该路径是否已存在（`readlink` 对不存在路径返回 `-ENOENT`）。最多重试 100 次。

若 `buf` 非空，将生成的路径复制到 `buf` 并返回 `buf`；若 `buf` 为 `NULL`，则使用静态内部缓冲区并返回其地址。

**警告**: 此函数在 POSIX.1-2008 中被标记为过时，存在 TOCTOU 竞态条件风险——在 `tmpnam` 返回和调用者实际创建文件之间，另一个进程可能创建同名文件。请使用 `mkstemp` 或 `tmpfile` 替代。

#### 前置条件

- 若 `buf` 非空：指向至少 `L_tmpnam` 字节的写空间
- `/tmp` 目录存在且可访问

#### 后置条件

- **Case 1 成功生成唯一文件名**
  - 返回指向包含 `/tmp/tmpnam_XXXXXX` 形式路径的缓冲区的指针（`buf` 或内部静态缓冲区）
  - 该路径在设计上不与当前已有文件冲突
  - 不创建文件

- **Case 2 所有尝试失败（100 次内无唯一文件名）**
  - 返回 `NULL`

#### 系统算法

```
tmpnam(buf):
  static internal[L_tmpnam]    // 内部静态缓冲区
  s[22] = "/tmp/tmpnam_XXXXXX" // 固定前缀 + 随机占位符
  
  for try in [0, MAXTRIES=100):
    1. __randname(s+12)            // 将 s[12..17] 替换为随机字母数字
    2. r = readlink(s, &(char[1]){0}, 1)  // 检测路径是否存在
       // readlink 在路径不存在时返回 -ENOENT
    3. if r == -ENOENT:            // 路径不存在 = 可用
         return strcpy(dest, s)    // dest = buf 或 internal
  
  return 0  // 所有尝试失败
```

#### 不变量

- 生成的路径名始终以 `/tmp/tmpnam_` 为前缀，后跟 6 个字母数字字符
- 内部静态缓冲区 `internal[L_tmpnam]` 在不带参数调用时被重用，非线程安全（不要求线程安全，该函数已过时）
- 最多 `MAXTRIES=100` 次尝试

#### 依赖

- `__randname(char *template)` — 生成随机文件名后缀（定义于 `src/temp/__randname.c`）
- `__syscall(SYS_readlink, ...)` / `__syscall(SYS_readlinkat, AT_FDCWD, ...)` — 读取符号链接目标值系统调用（此处用作文件存在检测）（定义于 `src/internal/syscall.h`）
- `strcpy(char *dst, const char *src)` — 字符串复制（来自 `<string.h>`）
- `L_tmpnam` — 临时文件名缓冲区最小长度常量（来自 `<stdio.h>`）
- `ENOENT` — 错误码：文件不存在（来自 `<errno.h>`）
- `AT_FDCWD` — 当前工作目录常量（来自 `<fcntl.h>`，条件编译分支）

#### 安全警告

此函数存在固有的 TOCTOU（Time-of-check-time-of-use）竞态条件：在 `tmpnam` 检测到路径不存在后和调用者创建文件之前的这段时间内，另一个进程可能创建同名文件。C 标准和 POSIX 均不推荐使用此函数。
