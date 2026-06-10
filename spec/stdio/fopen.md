# fopen.c 规约

> musl libc 标准库文件打开函数实现。根据指定的文件名和模式字符串打开文件，返回关联的 `FILE*` 流。

---

## 依赖图

```
fopen
  ├─> __fmodeflags  (see __fmodeflags.c spec — 将 mode 字符串转换为 open() 标志)
  ├─> sys_open      (see src/internal/syscall.h — 系统调用 open)
  ├─> __syscall     (see src/internal/syscall.h — 通用系统调用接口)
  ├─> __fdopen      (see __fdopen.c spec — 用文件描述符分配并初始化 FILE 对象)
  └─> strchr        (see src/string/strchr.c — 字符串首字符搜索)
```

---

## 函数规约

### 1. fopen

```c
FILE *fopen(const char *restrict filename, const char *restrict mode);
```

[Visibility]: User — 声明于 `<stdio.h>`，用户程序可直接调用

#### Intent

根据 `filename` 指定的路径和 `mode` 指定的访问模式，创建并打开一个带缓冲的标准 I/O 流。这是用户打开文件最常用的入口函数。

#### 前置条件

- `filename`: 一个以 NULL 结尾的有效路径字符串
- `mode`: 一个以 NULL 结尾的有效模式字符串，首字符必须为 `'r'`、`'w'` 或 `'a'`
- 可选 mode 后缀字符: `+`（读写）、`x`（排他创建）、`e`（close-on-exec）、`b`（二进制，无操作）

#### 后置条件

- **Case 1: 成功** — 返回指向新分配的 `FILE` 对象的指针
  - 文件描述符已通过 `sys_open(filename, flags, 0666)` 打开
  - 若 mode 中包含 `e`，且底层 `open()` 不支持 `O_CLOEXEC`，则通过 `fcntl(fd, F_SETFD, FD_CLOEXEC)` 额外设置 close-on-exec 标志
  - `FILE` 对象已通过 `__fdopen` 初始化，包含缓冲区、操作函数指针等
  - `FILE` 对象已通过 `__ofl_add` 注册到全局打开文件链表
- **Case 2: 失败** — 返回 `NULL`
  - 若 mode 首字符不合法，设置 `errno = EINVAL`
  - 若 `sys_open` 失败，保持底层系统的 `errno` 值
  - 若 `__fdopen` 分配/初始化失败，已关闭文件描述符（通过 `SYS_close`）

#### 系统算法

```
fopen(filename, mode):
  1. 校验 mode 首字符: 调用 strchr("rwa", *mode)，若为 NULL 则 errno=EINVAL, return NULL
  2. 调用 __fmodeflags(mode) 将模式字符串转换为 open(2) 标志位
  3. 调用 sys_open(filename, flags, 0666) 打开文件
     若 fd < 0, return NULL
  4. 若 flags & O_CLOEXEC, 调用 __syscall(SYS_fcntl, fd, F_SETFD, FD_CLOEXEC)
     （弥补不支持 atomic O_CLOEXEC 的内核）
  5. 调用 __fdopen(fd, mode) 创建 FILE 对象
     若成功, return f
  6. 清理: 调用 __syscall(SYS_close, fd); return NULL
```

#### 不变量

- 不会通过共享源路径的符号链接泄露控制权给其他进程（`O_CLOEXEC` 保证）
- 返回的 `FILE*` 在不再需要时必须由调用者通过 `fclose` 释放

#### 依赖

- `__fmodeflags(mode)` — 将 mode 字符串转换为 `open()` 标志位（定义于 `src/stdio/__fmodeflags.c`）
- `sys_open(filename, flags, mode)` — 系统调用 `open`（定义于 `src/internal/syscall.h`）
- `__syscall(SYS_fcntl, ...)` — 系统调用 `fcntl`（定义于 `src/internal/syscall.h`）
- `__fdopen(fd, mode)` — 从文件描述符创建 `FILE` 对象（定义于 `src/stdio/__fdopen.c`）
- `strchr("rwa", *mode)` — 字符串搜索（定义于 `src/string/strchr.c`）

#### 错误处理

| 条件 | errno 值 |
|------|----------|
| mode 首字符非 `r`/`w`/`a` | `EINVAL` |
| 文件打开失败 | 由 `sys_open` 设置（如 `ENOENT`, `EACCES` 等） |
| `__fdopen` 分配失败 | 由 `malloc` 设置（`ENOMEM`） |
