# \_\_stdio_read.c 规约

> musl libc 内部 FILE 默认读操作实现。作为 `f->read` 函数指针的默认值，通过 `readv`/`read` 系统调用从文件描述符读取数据，并提供缓冲管理。

---

## 依赖图

```
__stdio_read
  ├─> struct iovec   (<sys/uio.h>)
  ├─> syscall(SYS_readv, ...)   (内核)
  └─> syscall(SYS_read, ...)    (内核)
```

---

## 函数规约

### 1. \_\_stdio_read

```c
size_t __stdio_read(FILE *f, unsigned char *buf, size_t len);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->read` 函数指针的默认值，被 `__toread`、`__uflow` 等间接调用。

#### Intent

从文件描述符读取数据到用户缓冲区。当流有内部缓冲区时，使用 `readv` 执行一次分散读取——将尽可能多的数据读入用户缓冲区，将剩余数据读入内部缓冲区（为后续短读取做准备，如 `fgetc`）。若无内部缓冲区，退化为普通 `read`。

#### 前置条件

- `f`: `FILE*`，其 `fd` 为有效的文件描述符
- `buf`: `unsigned char*`，用户提供缓冲区，长度至少为 `len`
- `len`: 请求读取的字节数

#### 后置条件

**Case 1: 成功读取（cnt > 0）**

- 若无内部缓冲区（`f->buf_size == 0`）或读取数据全在 `buf` 内：
  - 返回 `min(cnt, len)`（实际读入 `buf` 的字节数）
- 若有内部缓冲区且 `readv` 将末尾数据填入 `buf`：
  - `f->rpos = f->buf`，`f->rend = f->buf + (cnt - iov[0].iov_len)`
  - 若有缓冲数据（`f->buf_size > 0`），`buf[len-1] = *f->rpos++`
  - 返回 `len`（用户缓冲区的 `len-1` 字节 + 内部缓冲区的 1 字节）
  - 注意：当 `len-1` 字节读入 `buf` 后，最后一个字节来自内部缓冲区，这是为了翻新 `rpos`/`rend` 指针以建立读取缓冲状态

**Case 2: 读取错误/EOF（cnt <= 0）**

- `cnt < 0`: 设置 `f->flags |= F_ERR`（读错误）
- `cnt == 0`: 设置 `f->flags |= F_EOF`（文件结束）
- 返回 `0`

#### 系统算法

```
__stdio_read(f, buf, len):
  /* 1. 构造 iovec：用户缓冲区 + 内部缓冲区 */
  iov[0] = { buf, len - !!f->buf_size }   // 预留1字节给内部缓冲区
  iov[1] = { f->buf, f->buf_size }        // 内部缓冲区

  /* 2. 系统调用 */
  if iov[0].iov_len > 0:
    cnt = syscall(SYS_readv, f->fd, iov, 2)
  else:
    cnt = syscall(SYS_read, f->fd, iov[1].iov_base, iov[1].iov_len)

  /* 3. 错误处理 */
  if cnt <= 0:
    f->flags |= (cnt ? F_ERR : F_EOF)
    return 0

  /* 4. 若无溢出到内部缓冲区，直接返回 */
  if cnt <= iov[0].iov_len:
    return cnt

  /* 5. 设置内部读取缓冲区指针 */
  cnt -= iov[0].iov_len
  f->rpos = f->buf
  f->rend = f->buf + cnt
  if f->buf_size != 0:
    buf[len-1] = *f->rpos++        // 回填最后一字节
  return len
```

#### 不变量

- `f->rpos` 和 `f->rend` 仅在 `cnt > iov[0].iov_len` 时被修改（即 `readv` 填入了内部缓冲区）
- `len - !!f->buf_size` 确保当流拥有内部缓冲区时，用户缓冲区末尾预留 1 字节给内部缓冲区交错
- 若有内部缓冲区（`f->buf_size > 0`），返回 `len` 时 `buf[len-1]` 被来自内部缓冲区的值覆盖

#### 依赖

- `struct iovec` — 散布/聚集 I/O 向量（`<sys/uio.h>`）
- `syscall(SYS_readv, ...)` — 分散读取系统调用（内核接口）
- `syscall(SYS_read, ...)` — 普通读取系统调用（内核接口）
- `F_ERR`, `F_EOF` — 文件流标志位（`stdio_impl.h`）
