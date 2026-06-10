# \_\_stdio_close.c 规约

> musl libc 内部 FILE 默认关闭操作实现。作为 `f->close` 函数指针的默认值，通过 `close` 系统调用关闭文件描述符，并在关闭前调用 AIO 清理回调。

---

## 依赖图

```
__stdio_close
  ├─> __aio_close  (aio_impl.h, 默认弱别名为 dummy)
  └─> syscall(SYS_close, ...)   (内核)
```

---

## 函数规约

### 1. dummy（static 辅助函数）

```c
static int dummy(int fd);
```

[Visibility]: Internal — `static` 函数，仅本文件可见。被 `__aio_close` 弱别名引用。

#### Intent

当 AIO 子系统未被链接时的默认实现，仅返回传入的 `fd`。

#### 前置条件

- `fd`: 有效的文件描述符

#### 后置条件

- 返回 `fd`（无操作）

---

### 2. \_\_stdio_close

```c
int __stdio_close(FILE *f);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->close` 函数指针的默认值，被 `fclose` 等间接调用。

#### Intent

关闭 `FILE` 关联的文件描述符。在关闭前，调用 `__aio_close(f->fd)` 以允许 AIO 子系统执行必要的清理（若 AIO 未被链接，则此为无操作）。

#### 前置条件

- `f`: `FILE*`，其 `fd` 为有效的文件描述符

#### 后置条件

**Case 1: 关闭成功**

- `f->fd` 引用的文件描述符被关闭
- `__aio_close(f->fd)` 已完成清理
- 返回 `syscall(SYS_close, ...)` 的返回值（通常为 `0`）

**Case 2: 关闭失败**

- 返回 `-1`，errno 由 `close` 系统调用设置

#### 系统算法

```
__stdio_close(f):
  /* 1. 运行 AIO 清理（若 AIO 子系统已链接，则为实际清理；否则为无操作） */
  aio_fd = __aio_close(f->fd)

  /* 2. 关闭文件描述符 */
  return syscall(SYS_close, aio_fd)
```

#### 不变量

- `__aio_close` 始终先于 `close` 系统调用执行
- `f` 的其他字段在关闭操作后可能变为无效（由调用方负责后续处理）

#### 依赖

- `__aio_close()` — AIO 关闭回调（`aio_impl.h`，默认弱别名为 `dummy`）
- `syscall(SYS_close, ...)` — 关闭文件描述符系统调用（内核接口）
