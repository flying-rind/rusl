# \_\_stdout_write.c 规约

> musl libc 内部 stdout 专用写函数实现。在首次写入 stdout 时，将 `f->write` 替换为 `__stdio_write`，并探测终端窗口大小以决定是否启用行缓冲模式。

---

## 依赖图

```
__stdout_write
  ├─> __stdio_write       (see __stdio_write.c spec)
  ├─> __syscall(SYS_ioctl, TIOCGWINSZ)   (内核)
  └─> struct winsize      (<sys/ioctl.h>)
```

---

## 函数规约

### 1. \_\_stdout_write

```c
size_t __stdout_write(FILE *f, const unsigned char *buf, size_t len);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。在 `__fdopen` 初始化时被设置为 stdout 的 `f->write` 函数指针。

#### Intent

stdout 的延迟初始化写函数。首次调用时：
1. 将 `f->write` 替换为 `__stdio_write`（此后不再执行初始化逻辑）
2. 除非流设置了 `F_SVB` 标志，否则通过 `ioctl(TIOCGWINSZ)` 检测 stdout 是否为终端：
   - 若 ioctl 成功（stdout 是终端）：保持 `f->lbf = '\n'`（行缓冲模式）
   - 若 ioctl 失败且非 `F_SVB`：设置 `f->lbf = -1`（关闭行缓冲，走全缓冲/无缓冲）
3. 调用 `__stdio_write` 执行实际写入

#### 前置条件

- `f`: `FILE*`，stdout 文件流
- `f->flags` 未设置 `F_SVB`（或已设置，则跳过 ioctl）
- `buf` / `len`: 同 `__stdio_write` 的参数要求

#### 后置条件

- 首次调用后 `f->write == __stdio_write`（后续调用不再进入此函数）
- 若 stdout 是终端且无 `F_SVB`：`f->lbf = '\n'`（行缓冲）
- 若 stdout 不是终端且无 `F_SVB`：`f->lbf = -1`（关闭行缓冲）
- 若 `F_SVB` 已设置：`f->lbf` 不变
- 返回值同 `__stdio_write(f, buf, len)`

#### 系统算法

```
__stdout_write(f, buf, len):
  /* 1. 覆盖 write 函数指针，延迟初始化只执行一次 */
  f->write = __stdio_write

  /* 2. 检测终端并配置行缓冲 */
  if !(f->flags & F_SVB):
    wsz: struct winsize
    if __syscall(SYS_ioctl, f->fd, TIOCGWINSZ, &wsz) != 0:  // ioctl 失败
      f->lbf = -1                      // 非终端，关闭行缓冲
    // ioctl 成功：保持 f->lbf = '\n'（行缓冲）

  /* 3. 执行实际写入 */
  return __stdio_write(f, buf, len)
```

#### 不变量

- 此函数指针在第一次调用后被替换为 `__stdio_write`，因此是幂等的（仅首次执行初始化逻辑）
- `F_SVB` 标志保护 `f->lbf` 不被 ioctl 探测覆盖

#### 依赖

- `__stdio_write()` — 默认写操作（本模块，see `__stdio_write.c` spec）
- `struct winsize` — 终端窗口大小（`<sys/ioctl.h>`）
- `__syscall(SYS_ioctl, ...)` — ioctl 系统调用（内核接口）
- `TIOCGWINSZ` — 获取窗口大小的 ioctl 请求码（`<sys/ioctl.h>`）
- `F_SVB` — 流标志位（`stdio_impl.h`）
