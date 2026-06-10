# fmemopen.c 规约

> musl libc 内存流打开函数实现。创建一个将内存缓冲区作为文件进行读写的 `FILE*` 流。

---

## 依赖图

```
fmemopen
  ├─> mem_FILE (struct)           (内部定义, 见下方)
  ├─> cookie (struct)             (内部定义, 见下方)
  ├─> mread (static)              (内部定义, 见下方)
  ├─> mwrite (static)             (内部定义, 见下方)
  ├─> mseek (static)              (内部定义, 见下方)
  ├─> mclose (static)             (内部定义, 见下方)
  ├─> __ofl_add                   (see ofl_add.c spec)
  ├─> malloc / memset / memcpy    (来自 <stdlib.h> / <string.h>)
  ├─> strchr / strnlen            (来自 <string.h>)
  └─> libc.threaded               (来自 "libc.h")
```

---

## 内部类型定义

### struct cookie

```c
struct cookie {
    size_t pos, len, size;
    unsigned char *buf;
    int mode;
};
```

[Visibility]: Internal (不导出) — 内存流的内部状态控制块

| 字段 | 类型 | 含义 |
|------|------|------|
| `pos` | `size_t` | 当前读写位置 |
| `len` | `size_t` | 有效数据长度 |
| `size` | `size_t` | 缓冲区总大小 |
| `buf` | `unsigned char *` | 指向用户提供的缓冲区或内部分配缓冲区的指针 |
| `mode` | `int` | 打开模式首字符（`'r'`/`'w'`/`'a'`） |

### struct mem_FILE

```c
struct mem_FILE {
    FILE f;
    struct cookie c;
    unsigned char buf[UNGET+BUFSIZ], buf2[];
};
```

[Visibility]: Internal (不导出) — 内存流 `FILE` 对象，包含标准 `FILE` 结构体、cookie 状态、以及灵活数组缓冲区 `buf2`（仅在用户未提供缓冲区时用作自动分配的数据存储）。

---

## 函数规约

### 1. fmemopen

```c
FILE *fmemopen(void *restrict buf, size_t size, const char *restrict mode);
```

[Visibility]: User — 声明于 `<stdio.h>`，POSIX.1-2008 标准接口，用户程序可直接调用

#### Intent

创建一个使用内存缓冲区进行 I/O 的 `FILE` 流。允许程序将内存当作文件读写，适用于需要内存中数据格式化的场景（如创建字符串缓冲区、在内存中处理文件数据等）。

#### 前置条件

- `mode`: 有效的模式字符串，首字符必须为 `'r'`、`'w'` 或 `'a'`；可选含 `'+'` 表示可读写
- `buf`: 用户提供的缓冲区指针（可为 `NULL`，此时 musl 内部分配）
- `size`: 缓冲区大小（若 `buf != NULL` 则为用户提供的缓冲区大小；若 `buf == NULL` 则 musl 分配 `size` 字节）
- 若 `buf == NULL` 且 `size > PTRDIFF_MAX`，设置 `errno = ENOMEM` 并返回 `NULL`

#### 后置条件

- **Case 1: 成功** — 返回指向新创建的 `FILE` 对象的指针
  - `FILE` 的 `fd` 设置为 `-1`（表示无底层文件描述符）
  - `FILE` 的 `read`、`write`、`seek`、`close` 函数指针设置为内部实现（`mread`、`mwrite`、`mseek`、`mclose`）
  - 若 mode 不含 `'+'`，设置只读限制标志（`F_NOWR` 或 `F_NORD`）
  - 若 `buf == NULL`，内部分配 `size` 字节缓冲区并零填充
  - 若 mode 为 `'r'`，`c.len = size`（文件内容为整个缓冲区初始内容）
  - 若 mode 为 `'a'`，`c.len = c.pos = strnlen(buf, size)`（从末尾开始追加），且如果是 `'+'` 模式设置 `*buf = 0`（确保字符串终止）
  - 若 `'+'` 模式且 mode 非 `'r'` 非 `'a'`（即 `'w+'`），设置 `*c.buf = 0`（创建一个空字符串）
  - 若未启用线程化（`!libc.threaded`），`f->lock = -1`（禁用锁机制）
  - `FILE` 通过 `__ofl_add` 注册到全局打开文件链表
- **Case 2: 失败** — 返回 `NULL`
  - 若 mode 首字符不合法：`errno = EINVAL`
  - 若 `buf == NULL` 且 `size > PTRDIFF_MAX`：`errno = ENOMEM`
  - 若 `malloc` 失败：保持 `malloc` 设置的 `errno`

#### 系统算法

```
fmemopen(buf, size, mode):
  1. plus = !!strchr(mode, '+')
  2. 校验 mode 首字符: strchr("rwa", *mode) != NULL, 否则 errno=EINVAL, return NULL
  3. 若 !buf 且 size > PTRDIFF_MAX: errno=ENOMEM, return NULL
  4. 分配 mem_FILE: f = malloc(sizeof(*f) + (buf ? 0 : size))
     若 !f: return NULL
  5. memset(f, 0, offsetof(struct mem_FILE, buf))  // 仅清零结构体部分，不碰缓冲区
  6. 初始化 FILE 字段:
        f->f.cookie = &f->c
        f->f.fd = -1
        f->f.lbf = EOF
        f->f.buf = f->buf + UNGET     // 用户缓冲空间前预留 UNGET 字节
        f->f.buf_size = sizeof f->buf - UNGET
  7. 若 !buf: buf = f->buf2; memset(buf, 0, size)  // 使用内部分配缓冲区
  8. 初始化 cookie:
        c.buf = buf; c.size = size; c.mode = *mode
  9. 设置文件标志和初始状态:
        若 !plus: f->f.flags = (*mode == 'r') ? F_NOWR : F_NORD
        若 *mode == 'r': c.len = size          // 只读: 内容 = 整个缓冲区
        若 *mode == 'a': c.len = c.pos = strnlen(buf, size)  // 追加: 位置在末尾
            若 plus: *c.buf = 0                // a+ 模式: 确保字符串终止
        若 plus 且 *mode != 'r' 且 *mode != 'a': *c.buf = 0  // w+ 模式: 创建空字符串
  10. 设置操作函数指针:
        f->f.read  = mread
        f->f.write = mwrite
        f->f.seek  = mseek
        f->f.close = mclose
  11. 若 !libc.threaded: f->f.lock = -1
  12. return __ofl_add(&f->f)
```

#### 不变量

- `fd` 始终为 `-1`（无底层文件描述符，所有 I/O 操作直接操作内存）
- 内存流不支持行缓冲（`lbf = EOF`）
- 关闭操作（`mclose`）不释放 `buf` 本身（调用者负责管理用户提供的缓冲区）
- 当 `+` 模式激活时，`fmemopen` 始终尝试维护一个以 NULL 结尾的字符串（对于文本处理很重要）

---

### 2. mread (static)

```c
static size_t mread(FILE *f, unsigned char *buf, size_t len);
```

[Visibility]: Internal (不导出) — 内存流内部读取函数

#### Intent

从内存流的缓冲区中读取数据，维护当前位置指针 (`cookie.pos`)，并将预读数据加载到 `FILE` 的 `rpos`/`rend` 缓冲区中。

#### 前置条件

- `f` 指向有效的 `mem_FILE` 对象
- `buf` 指向有效的输出缓冲区

#### 后置条件

- 读取 `min(len, c.len - c.pos)` 字节数据到 `buf`
- 更新 `c.pos` 增加已读取字节数
- 若可读数据不足 `len`，设置 `f->flags |= F_EOF`
- 预读缓冲区填充：将 `min(remaining, f->buf_size)` 字节预读进 `f->rpos`/`f->rend`
- 返回实际读取的字节数

#### 系统算法

```
mread(f, buf, len):
  c = f->cookie
  rem = c->len - c->pos                       // 剩余可读字节数
  if (c->pos > c->len) rem = 0                // 防御: 位置超出范围
  if (len > rem):
    len = rem                                  // 截断到可用数据
    f->flags |= F_EOF                          // 设置 EOF 标志
  memcpy(buf, c->buf + c->pos, len)            // 拷贝请求的数据
  c->pos += len                                // 更新位置
  rem -= len
  if (rem > f->buf_size) rem = f->buf_size     // 预读量不超过 buf_size
  f->rpos = f->buf                             // 设置读缓冲指针
  f->rend = f->buf + rem
  memcpy(f->rpos, c->buf + c->pos, rem)        // 预读到 FILE 缓冲区
  c->pos += rem                                // 更新位置
  return len
```

---

### 3. mwrite (static)

```c
static size_t mwrite(FILE *f, const unsigned char *buf, size_t len);
```

[Visibility]: Internal (不导出) — 内存流内部写入函数

#### Intent

将数据写入内存流的缓冲区。先刷新 `FILE` 自身的写入缓冲区中的任何待写数据，然后将新数据直接写入 `cookie.buf`。支持 `'a'` 模式下的追加行为。

#### 前置条件

- `f` 指向有效的 `mem_FILE` 对象
- `buf` 指向有效的数据源

#### 后置条件

- 写入 `min(len, c.size - c.pos)` 字节数据到 `c.buf`
- 若 `c.mode == 'a'`，写入前将 `c.pos` 定位到 `c.len`（支持追加）
- 更新 `c.pos`；若 `c.pos > c.len`，更新 `c.len`
- 若 `c.len < c.size`，维护 NULL 终止符：`c.buf[c.len] = 0`
- 若缓冲区已满，且非 `F_NORD` 模式且有空间，保留最后一个字节写入 NULL 终止符
- 返回实际写入的字节数

#### 系统算法

```
mwrite(f, buf, len):
  c = f->cookie
  len2 = f->wpos - f->wbase                    // FILE 写入缓冲区中的待写数据
  if (len2):
    f->wpos = f->wbase                          // 重置 wpos
    if (mwrite(f, f->wpos, len2) < len2) return 0  // 递归先写出待写数据

  if (c->mode == 'a') c->pos = c->len           // 追加模式: 定位到末尾
  rem = c->size - c->pos                        // 剩余可写空间
  if (len > rem) len = rem                      // 截断到可用空间
  memcpy(c->buf + c->pos, buf, len)
  c->pos += len
  if (c->pos > c->len):
    c->len = c->pos                             // 更新有效数据长度
    if (c->len < c->size): c->buf[c->len] = 0   // 维护 NULL 终止
    else if ((f->flags & F_NORD) && c->size) c->buf[c->size-1] = 0  // 末尾零
  return len
```

---

### 4. mseek (static)

```c
static off_t mseek(FILE *f, off_t off, int whence);
```

[Visibility]: Internal (不导出) — 内存流内部 seek 函数

#### Intent

在内存流缓冲区中移动读写位置。支持标准的 `SEEK_SET`/`SEEK_CUR`/`SEEK_END` 偏移原点。

#### 前置条件

- `f` 指向有效的 `mem_FILE` 对象
- `whence` 为 `0`（`SEEK_SET`）、`1`（`SEEK_CUR`）或 `2`（`SEEK_END`）

#### 后置条件

- **Case 1: 成功** — 更新 `c->pos` 到目标位置，返回新位置
- **Case 2: 失败** — `errno = EINVAL`, 返回 `-1`
  - 若 `whence > 2`
  - 若 `off < -base` 或 `off > (ssize_t)c.size - base`（越界）

#### 系统算法

```
mseek(f, off, whence):
  c = f->cookie
  if (whence > 2): errno=EINVAL, return -1
  base = (size_t[3]){0, c->pos, c->len}[whence]  // SEEK_SET=0, SEEK_CUR=pos, SEEK_END=len
  if (off < -base || off > (ssize_t)c.size - base):
    errno=EINVAL, return -1
  return c->pos = base + off
```

---

### 5. mclose (static)

```c
static int mclose(FILE *m);
```

[Visibility]: Internal (不导出) — 内存流内部关闭函数

#### Intent

内存流的关闭操作。始终成功返回 `0`。

#### 前置条件

- `m` 指向有效的 `mem_FILE` 对象

#### 后置条件

- **始终**: 返回 `0`
- 不释放 `cookie.buf` 内存（由调用者负责管理用户提供的缓冲区；对于 `fmemopen` 内部分配的缓冲区，其生命周期与 `mem_FILE` 绑定，由 `fclose` 处理）
