# \_\_stdio_write.c 规约

> musl libc 内部 FILE 默认写操作实现。作为 `f->write` 函数指针的默认值，通过 `writev` 系统调用将内部缓冲区和用户数据一并写入文件描述符，支持部分写入重试。

---

## 依赖图

```
__stdio_write
  ├─> struct iovec   (<sys/uio.h>)
  └─> syscall(SYS_writev, ...)   (内核)
```

---

## 函数规约

### 1. \_\_stdio_write

```c
size_t __stdio_write(FILE *f, const unsigned char *buf, size_t len);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->write` 函数指针的默认值，被 `__overflow`、`fflush` 等间接调用。

#### Intent

将内部写缓冲区的剩余数据（`f->wbase` 到 `f->wpos`）和用户数据（`buf[0..len-1]`）通过一次 `writev` 系统调用一并写出。若写出不完整，循环重试剩余部分直到全部写出或遇到错误。

#### 前置条件

- `f`: `FILE*`，其 `fd` 为有效的文件描述符
- `buf`: `const unsigned char*`，用户提供的数据，长度至少为 `len`
- `len`: 要写入的字节数
- `f->wbase` 和 `f->wpos` 在首次写入时为有效指针

#### 后置条件

**Case 1: 完全成功写入（cnt == rem）**

- `f->wend = f->buf + f->buf_size`
- `f->wpos = f->wbase = f->buf`（写缓冲区指针重置）
- 返回 `len`（用户数据全部写出）

**Case 2: 部分写入后错误（cnt < 0）**

- `f->wpos = f->wbase = f->wend = 0`（写缓冲区指针清零，防止使用无效数据）
- `f->flags |= F_ERR`
- 若 `iovcnt == 2`（内部缓冲区和用户数据都未写完）：返回 `0`
- 若 `iovcnt == 1`（内部缓冲区已写完，仅剩用户数据）：返回 `len - iov[0].iov_len`（已写出的用户数据字节数）

**Case 3: 部分写入，无错误（0 < cnt < rem）**

- 更新 `iov` 和 `rem` 以反映剩余待写数据
- 循环重试 `writev`
- 注意：内部缓冲区（`iov[0]`）的数据优先写出

#### 不变量

- 无论成功或失败，`buf` 内容不被修改
- `f->flags` 仅在发生错误时修改（设置 `F_ERR`）
- 写缓冲区指针仅在完全写出后重置为有效值，或在错误时清零以标记无效状态
- 错误返回时，返回值表示已从用户缓冲区 `buf` 中成功写出的字节数

#### 系统算法

```
__stdio_write(f, buf, len):
  /* 1. 构造 iovec */
  iovs[0] = { f->wbase, f->wpos - f->wbase }     // 内部缓冲区剩余数据
  iovs[1] = { buf, len }                          // 用户数据
  iov = iovs
  rem = iov[0].iov_len + iov[1].iov_len
  iovcnt = 2

  /* 2. 跳过空的内部缓冲区 */
  if iov[0].iov_len == 0:
    iov++; iovcnt--

  /* 3. 循环写出直到完成或错误 */
  loop:
    cnt = syscall(SYS_writev, f->fd, iov, iovcnt)

    if cnt == rem:                                 // 完全写出
      f->wend = f->buf + f->buf_size
      f->wpos = f->wbase = f->buf
      return len

    if cnt < 0:                                    // 写出错误
      f->wpos = f->wbase = f->wend = 0
      f->flags |= F_ERR
      return (iovcnt == 2) ? 0 : (len - iov[0].iov_len)

    /* 部分写出，调整 iov */
    rem -= cnt
    if cnt > iov[0].iov_len:
      cnt -= iov[0].iov_len
      iov++; iovcnt--
    iov[0].iov_base += cnt
    iov[0].iov_len -= cnt
```

#### 依赖

- `struct iovec` — 散布/聚集 I/O 向量（`<sys/uio.h>`）
- `syscall(SYS_writev, ...)` — 聚集写入系统调用（内核接口）
- `F_ERR` — 文件流标志位（`stdio_impl.h`）
