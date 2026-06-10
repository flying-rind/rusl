# fopencookie.c 规约

> musl libc 自定义回调流打开函数实现。创建一个 `FILE*` 流，其底层 I/O 操作由用户提供的回调函数实现。这是 GNU 扩展接口。

---

## 依赖图

```
fopencookie
  ├─> cookie_FILE (struct)      (内部定义, 见下方)
  ├─> fcookie (struct)          (内部定义, 见下方)
  ├─> cookieread (static)       (内部定义, 见下方)
  ├─> cookiewrite (static)      (内部定义, 见下方)
  ├─> cookieseek (static)       (内部定义, 见下方)
  ├─> cookieclose (static)      (内部定义, 见下方)
  ├─> __ofl_add                 (see ofl_add.c spec)
  ├─> malloc / memset           (来自 <stdlib.h> / <string.h>)
  ├─> strchr                    (来自 <string.h>)
  └─> cookie_io_functions_t     (来自 <stdio.h>)
```

---

## 外部类型

### cookie_io_functions_t

定义于 `<stdio.h>`:

```c
typedef ssize_t (cookie_read_function_t)(void *, char *, size_t);
typedef ssize_t (cookie_write_function_t)(void *, const char *, size_t);
typedef int (cookie_seek_function_t)(void *, off_t *, int);
typedef int (cookie_close_function_t)(void *);

typedef struct _IO_cookie_io_functions_t {
    cookie_read_function_t  *read;
    cookie_write_function_t *write;
    cookie_seek_function_t  *seek;
    cookie_close_function_t *close;
} cookie_io_functions_t;
```

---

## 内部类型定义

### struct fcookie

```c
struct fcookie {
    void *cookie;
    cookie_io_functions_t iofuncs;
};
```

[Visibility]: Internal (不导出) — 自定义回调流的内部状态

| 字段 | 类型 | 含义 |
|------|------|------|
| `cookie` | `void *` | 用户提供的不透明 cookie，传递给所有回调函数 |
| `iofuncs` | `cookie_io_functions_t` | 用户提供的 I/O 回调函数集合 |

### struct cookie_FILE

```c
struct cookie_FILE {
    FILE f;
    struct fcookie fc;
    unsigned char buf[UNGET+BUFSIZ];
};
```

[Visibility]: Internal (不导出) — 自定义回调流 `FILE` 对象，包含标准 `FILE` 结构体、fcookie 状态和缓冲区。

---

## 函数规约

### 1. fopencookie

```c
FILE *fopencookie(void *cookie, const char *mode, cookie_io_functions_t iofuncs);
```

[Visibility]: User — 声明于 `<stdio.h>`（需定义 `_GNU_SOURCE`），GNU 扩展接口，用户程序可直接调用

#### Intent

创建一个 `FILE*` 流，其所有底层 I/O 操作都由用户提供的回调函数执行。`cookie` 参数是不透明的用户数据，被传递给每个回调函数。这允许程序将任意数据源或目标伪装为文件流，实现自定义 I/O 后端（如网络流、压缩流等）。

#### 前置条件

- `mode`: 有效的模式字符串，首字符必须为 `'r'`、`'w'` 或 `'a'`；可选含 `'+'` 表示可读写
- `iofuncs`: 用户提供的回调函数集合，各个函数可以为 `NULL`（表示该操作不被支持）
- `cookie`: 不透明用户数据指针（可为任意值，被传递给所有回调）

#### 后置条件

- **Case 1: 成功** — 返回新创建的 `FILE*` 对象
  - `FILE` 的 `fd` 设为 `-1`（无底层文件描述符）
  - `FILE` 的 `cookie` 字段指向内部 `fcookie` 结构
  - `FILE` 的 `read`、`write`、`seek`、`close` 函数指针设置为内部封装函数（`cookieread`、`cookiewrite`、`cookieseek`、`cookieclose`）
  - 内部封装函数会调用用户提供的回调（若为非 `NULL`）或将操作转换为适当的默认行为
  - 若 mode 不含 `'+'`，设置只读/只写限制标志（`F_NOWR` 或 `F_NORD`）
  - `buf_size = BUFSIZ`（`sizeof f->buf - UNGET`），支持用户缓冲
  - 行缓冲被禁用（`lbf = EOF`）
  - 通过 `__ofl_add` 注册到全局打开文件链表
- **Case 2: 失败** — 返回 `NULL`
  - 若 mode 首字符不合法：`errno = EINVAL`
  - 若 `malloc` 分配失败：`errno = ENOMEM`

#### 系统算法

```
fopencookie(cookie, mode, iofuncs):
  1. 校验 mode 首字符:
        if (!strchr("rwa", *mode)): errno=EINVAL, return NULL
  2. 分配 cookie_FILE:
        f = malloc(sizeof *f)
        if (!f) return NULL
  3. memset(&f->f, 0, sizeof f->f)          // 仅清零 FILE 结构，不碰缓冲区
  4. 若 mode 不含 '+':
        f->f.flags = (*mode == 'r') ? F_NOWR : F_NORD
  5. 设置 fcookie:
        f->fc.cookie = cookie
        f->fc.iofuncs = iofuncs
  6. 初始化 FILE:
        f->f.fd = -1
        f->f.cookie = &f->fc                 // FILE.cookie 指向 fcookie
        f->f.buf = f->buf + UNGET            // 用户缓冲空间前预留 UNGET 字节
        f->f.buf_size = sizeof f->buf - UNGET
        f->f.lbf = EOF
  7. 设置操作函数指针:
        f->f.read  = cookieread
        f->f.write = cookiewrite
        f->f.seek  = cookieseek
        f->f.close = cookieclose
  8. return __ofl_add(&f->f)                // 注册到全局打开文件链表
```

#### 不变量

- `fd` 始终为 `-1`（无底层文件描述符）
- 不支持行缓冲（`lbf = EOF`）
- 关闭操作（`cookieclose`）调用用户提供的 `close` 回调（若为 `NULL` 则返回 0）

---

### 2. cookieread (static)

```c
static size_t cookieread(FILE *f, unsigned char *buf, size_t len);
```

[Visibility]: Internal (不导出) — 封装用户读取回调

#### Intent

从自定义数据源读取数据到 FILE 缓冲区。若用户的 `read` 回调为 `NULL`，立即设置 `EOF`。否则，先尝试一次性读取 `len - !!f->buf_size` 字节（实际量由用户回调决定），然后用剩余空间将数据预读进 FILE 内部缓冲区。

#### 前置条件

- `f` 指向有效的 `cookie_FILE` 对象
- `buf` 指向有效的输出缓冲区
- `len` > 0

#### 后置条件

- **Case 1: 用户的 read 回调为 NULL** — 设置 `F_EOF`，返回 0
- **Case 2: 回调返回 0** — 设置 `F_EOF`，返回已读取字节数
- **Case 3: 回调返回 < 0** — 设置 `F_ERR`，返回已读取字节数
- **Case 4: 成功** — 返回实际读取字节数（包括预读到 FILE 缓冲区的一字节）

#### 系统算法

```
cookieread(f, buf, len):
  fc = f->cookie
  ret = -1; remain = len; readlen = 0
  len2 = len - !!f->buf_size                // 为预读预留 1 字节（若 buf_size > 0）

  if (!fc->iofuncs.read) goto bail          // 无读取回调: EOF

  if (len2):                                // 先做一次大读取
    ret = fc->iofuncs.read(cookie, buf, len2)
    if (ret <= 0) goto bail                 // 错误或 EOF
    readlen += ret; remain -= ret

  if (!f->buf_size || remain > !!f->buf_size) return readlen  // 不需要预读

  // 预读 1 字节到 FILE 内部缓冲区
  f->rpos = f->buf
  ret = fc->iofuncs.read(cookie, f->rpos, f->buf_size)
  if (ret <= 0) goto bail
  f->rend = f->rpos + ret
  buf[readlen++] = *f->rpos++               // 返回预读的第一个字节
  return readlen

bail:
  f->flags |= ret == 0 ? F_EOF : F_ERR
  f->rpos = f->rend = f->buf
  return readlen
```

---

### 3. cookiewrite (static)

```c
static size_t cookiewrite(FILE *f, const unsigned char *buf, size_t len);
```

[Visibility]: Internal (不导出) — 封装用户写入回调

#### Intent

将数据写入自定义数据目标。若用户的 `write` 回调为 `NULL`，返回 `len`（假装成功，实际丢弃数据）。否则先刷新 FILE 自身的写入缓冲区，再调用用户回调。

#### 前置条件

- `f` 指向有效的 `cookie_FILE` 对象
- `buf` 指向有效数据源

#### 后置条件

- **Case 1: 用户的 write 回调为 NULL** — 返回 `len`（数据被静默丢弃）
- **Case 2: 回调返回 < 0** — 清零写入缓冲区，设置 `F_ERR`，返回 0
- **Case 3: 成功** — 返回回调实际写入的字节数

#### 系统算法

```
cookiewrite(f, buf, len):
  fc = f->cookie
  len2 = f->wpos - f->wbase                // FILE 缓冲区中的待写数据
  if (!fc->iofuncs.write) return len       // 无写入回调: 假装写入成功
  if (len2):                               // 先刷新 FILE 缓冲区
    f->wpos = f->wbase
    if (cookiewrite(f, f->wpos, len2) < len2) return 0
  ret = fc->iofuncs.write(cookie, buf, len)
  if (ret < 0):
    f->wpos = f->wbase = f->wend = 0      // 错误时重置所有写指针
    f->flags |= F_ERR
    return 0
  return ret
```

---

### 4. cookieseek (static)

```c
static off_t cookieseek(FILE *f, off_t off, int whence);
```

[Visibility]: Internal (不导出) — 封装用户 seek 回调

#### Intent

在自定义数据流中移动位置。若用户的 `seek` 回调为 `NULL`，设置 `errno = ENOTSUP` 并返回 `-1`。否则调用用户回调并返回新文件偏移量。

#### 前置条件

- `f` 指向有效的 `cookie_FILE` 对象
- `whence` 为 `0`（SEEK_SET）、`1`（SEEK_CUR）或 `2`（SEEK_END）

#### 后置条件

- **Case 1: whence > 2** — `errno = EINVAL`, 返回 `-1`
- **Case 2: 用户 seek 回调为 NULL** — `errno = ENOTSUP`, 返回 `-1`
- **Case 3: 回调返回 < 0** — 返回该错误值
- **Case 4: 成功** — 返回 `off`（用户的 seek 回调通过 `&off` 指针参数返回新的偏移量）

#### 系统算法

```
cookieseek(f, off, whence):
  fc = f->cookie
  if (whence > 2): errno=EINVAL, return -1
  if (!fc->iofuncs.seek): errno=ENOTSUP, return -1
  res = fc->iofuncs.seek(cookie, &off, whence)  // 用户回调修改 off
  if (res < 0) return res
  return off                                     // 返回用户回调设置的偏移量
```

---

### 5. cookieclose (static)

```c
static int cookieclose(FILE *f);
```

[Visibility]: Internal (不导出) — 封装用户关闭回调

#### Intent

关闭自定义数据流。若用户提供了 `close` 回调则调用之；否则返回 `0`。

#### 前置条件

- `f` 指向有效的 `cookie_FILE` 对象

#### 后置条件

- **Case 1: 用户的 close 回调为非 NULL** — 调用 `fc->iofuncs.close(fc->cookie)`，返回其返回值
- **Case 2: 用户的 close 回调为 NULL** — 返回 `0`
