# aio 模块 — 对外导出 API 清单

> 本文件记录 `musl-1.2.6/src/aio/` 模块对外导出的所有用户可见接口（由 `<aio.h>` 声明）。

---

## Public API（用户直接可见）

| 函数签名 | 定义文件 | 标准 |
|----------|----------|------|
| `int aio_read(struct aiocb *);` | `aio.c` | POSIX.1-2001 |
| `int aio_write(struct aiocb *);` | `aio.c` | POSIX.1-2001 |
| `int aio_error(const struct aiocb *);` | `aio.c` | POSIX.1-2001 |
| `ssize_t aio_return(struct aiocb *);` | `aio.c` | POSIX.1-2001 |
| `int aio_cancel(int, struct aiocb *);` | `aio.c` | POSIX.1-2001 |
| `int aio_suspend(const struct aiocb *const [], int, const struct timespec *);` | `aio_suspend.c` | POSIX.1-2001 |
| `int aio_fsync(int, struct aiocb *);` | `aio.c` | POSIX.1-2001 |
| `int lio_listio(int, struct aiocb *__restrict const *__restrict, int, struct sigevent *__restrict);` | `lio_listio.c` | POSIX.1-2001 |

## 公共数据类型（用户直接可见）

| 类型名称 | 定义位置 | 标准 |
|----------|----------|------|
| `struct aiocb` | `<aio.h>` | POSIX.1-2001 |

## 公共常量（用户直接可见）

| 常量 | 值 | 定义位置 |
|------|-----|----------|
| `AIO_CANCELED` | `0` | `<aio.h>` |
| `AIO_NOTCANCELED` | `1` | `<aio.h>` |
| `AIO_ALLDONE` | `2` | `<aio.h>` |
| `LIO_READ` | `0` | `<aio.h>` |
| `LIO_WRITE` | `1` | `<aio.h>` |
| `LIO_NOP` | `2` | `<aio.h>` |
| `LIO_WAIT` | `0` | `<aio.h>` |
| `LIO_NOWAIT` | `1` | `<aio.h>` |

---

## Internal 符号（模块间共享，非用户 API）

| 符号 | 类型 | 定义文件 | 声明位置 |
|------|------|----------|----------|
| `__aio_fut` | `volatile int` | `aio.c` | `internal/aio_impl.h` |
| `__aio_close(int)` | `int (int)` | `aio.c` | `internal/aio_impl.h` |
| `__aio_atfork(int)` | `void (int)` | `aio.c` | `internal/aio_impl.h` |

注：`__aio_close` 由 `close()` 调用以取消关联文件描述符上的未完成 AIO 操作；`__aio_atfork` 由 `fork()` 调用以处理 fork 后的 AIO 状态清理。
