# freopen.c 规约

> musl libc 标准库文件重定向函数实现。将已有 `FILE` 流重定向到新文件路径，或修改已打开文件的访问模式。

---

## 依赖图

```
freopen
  ├─> __fmodeflags      (see __fmodeflags.c spec)
  ├─> FLOCK(f) / FUNLOCK(f) (宏, see stdio_impl.h)
  ├─> fflush(f)         (see fflush.c spec)
  ├─> fopen(filename, mode) (see fopen.c spec) [仅当 filename != NULL]
  ├─> __dup3            (see src/internal/syscall.h)
  ├─> fclose(f2)        (see fclose.c spec)
  ├─> fclose(f)         (see fclose.c spec) [失败路径]
  ├─> __syscall(SYS_fcntl, ...) (syscall 接口)
  └─> syscall(SYS_fcntl, ...)  (syscall 接口) [仅当 filename == NULL]
```

---

## 函数规约

### 1. freopen

```c
FILE *freopen(const char *restrict filename, const char *restrict mode, FILE *restrict f);
```

[Visibility]: User — 声明于 `<stdio.h>`，用户程序可直接调用

#### Intent

将一个已存在的 `FILE` 流重定向到另一个文件或修改其模式。函数首先刷新 `f` 的缓冲区并关闭其当前关联，然后将 `f` 与 `filename`（若提供）的新文件关联，或将 `f` 的模式修改为 `mode`（若 `filename == NULL`）。成功时返回原始 `f` 指针（重定向后的），失败时返回 `NULL` 且原始 `f` 被关闭。

#### 前置条件

- `mode`: 有效的模式字符串，首字符为 `'r'`、`'w'` 或 `'a'`
- `f`: 一个有效的已打开 `FILE*` 指针（不能为 `NULL`）
- 若 `filename != NULL`: 该路径必须为有效文件路径
- 若 `filename == NULL`: 只修改 `f` 对应文件描述符的访问模式标志（`fcntl`）

#### 后置条件

- **Case 1: filename != NULL 且操作成功**
  - `fflush(f)` 被调用以刷新当前缓冲区
  - 通过 `fopen(filename, mode)` 创建一个新的 `FILE` 对象 `f2`
  - 若 `f2->fd == f->fd`，设 `f2->fd = -1`（防止 `fclose(f2)` 时误关相同 fd）
  - 否则调用 `__dup3(f2->fd, f->fd, fl & O_CLOEXEC)` 将新文件描述符复制到 `f->fd`
  - 将 `f2` 的操作属性复制到 `f`：`flags`（保留 `F_PERM`）、`read`、`write`、`seek`、`close`
  - `fclose(f2)` 释放临时 `FILE` 对象
  - 重置 `f->mode = 0` 和 `f->locale = 0`
  - 返回 `f`

- **Case 2: filename == NULL 且操作成功**
  - `fflush(f)` 刷新缓冲区
  - 若 mode 含 `e`，设置 close-on-exec 标志
  - 调用 `fcntl(f->fd, F_SETFL, fl)` 修改文件描述符的访问模式
  - 返回 `f`

- **Case 3: 操作失败**
  - 关闭原始 `f`（调用 `fclose(f)`）
  - 返回 `NULL`

#### 系统算法

```
freopen(filename, mode, f):
  fl = __fmodeflags(mode)
  FLOCK(f)                           // 锁定 FILE 对象
  fflush(f)                          // 刷新当前缓冲区

  if (!filename):                    // 无新文件: 修改当前 fd 的模式
    if (fl & O_CLOEXEC)
      __syscall(SYS_fcntl, f->fd, F_SETFD, FD_CLOEXEC)
    fl &= ~(O_CREAT|O_EXCL|O_CLOEXEC)
    if (syscall(SYS_fcntl, f->fd, F_SETFL, fl) < 0)
      goto fail
  else:                              // 有新文件路径
    f2 = fopen(filename, mode)       // 打开新文件
    if (!f2) goto fail
    if (f2->fd == f->fd)             // 相同 fd 去重
      f2->fd = -1
    else if (__dup3(f2->fd, f->fd, fl & O_CLOEXEC) < 0)
      goto fail2
    // 将 f2 的操作属性移植到 f
    f->flags  = (f->flags & F_PERM) | f2->flags
    f->read   = f2->read
    f->write  = f2->write
    f->seek   = f2->seek
    f->close  = f2->close
    fclose(f2)                       // 释放临时的 f2
                                     // 注意: 若 f2->fd == -1, close 回调不执行

  f->mode = 0
  f->locale = 0
  FUNLOCK(f)
  return f

fail2:
  fclose(f2)
fail:
  fclose(f)                          // 失败时关闭原始 f
  return NULL
```

#### 不变量

- 在任何条件下，操作最终不会泄露文件描述符或 `FILE` 对象
- `F_PERM` 标志始终保留（若原始流是一个永久流如 stdout）
- 失败时原始 `f` 被关闭，这与 glibc 的语义略有不同（glibc 失败时保留原始 `f`）

#### 依赖

- `__fmodeflags(mode)` — mode 到 open() 标志的转换（定义于 `src/stdio/__fmodeflags.c`）
- `FLOCK(f)` / `FUNLOCK(f)` — FILE 对象锁/解锁（宏，定义于 `src/internal/stdio_impl.h`）
- `fflush(f)` — 流缓冲区刷新（定义于 `src/stdio/fflush.c`）
- `fopen(filename, mode)` — 打开新文件（定义于 `src/stdio/fopen.c`）
- `fclose(f)` — 关闭 FILE 流（定义于 `src/stdio/fclose.c`）
- `__dup3(newfd, oldfd, flags)` — 复制文件描述符（系统调用接口，定义于 `src/internal/syscall.h`）
- `__syscall(SYS_fcntl, ...)` — fcntl 系统调用（定义于 `src/internal/syscall.h`）

#### 错误处理

| 条件 | 行为 |
|------|------|
| mode 首字符不合法 | `fopen` 内部设置 `errno = EINVAL`，跳转到 fail，关闭 `f` |
| `filename != NULL` 且文件打开失败 | `fopen` 返回 `NULL`，跳转到 fail，关闭 `f` |
| `__dup3` 失败 | 跳转到 fail2，关闭 `f2`，再关闭 `f` |
| `filename == NULL` 且 `fcntl` 失败 | 跳转到 fail，关闭 `f` |
