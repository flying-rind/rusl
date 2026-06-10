# open_memstream.c 规约

> musl libc 动态内存流打开函数实现。创建一个自动增长的写入流，允许程序动态构建内存中的字符串缓冲区，并通过输出参数获取最终缓冲区指针和大小。

---

## 依赖图

```
open_memstream
  ├─> ms_FILE (struct)          (内部定义, 见下方)
  ├─> cookie (struct)           (内部定义, 见下方)
  ├─> ms_seek (static)          (内部定义, 见下方)
  ├─> ms_write (static)         (内部定义, 见下方)
  ├─> ms_close (static)         (内部定义, 见下方)
  ├─> __ofl_add                 (see ofl_add.c spec)
  ├─> malloc / realloc / free   (来自 <stdlib.h>)
  ├─> memcpy / memset           (来自 <string.h>)
  └─> libc.threaded             (来自 "libc.h")
```

---

## 内部类型定义

### struct cookie

```c
struct cookie {
    char **bufp;
    size_t *sizep;
    size_t pos;
    char *buf;
    size_t len;
    size_t space;
};
```

[Visibility]: Internal (不导出) — 动态内存流的内部状态控制块

| 字段 | 类型 | 含义 |
|------|------|------|
| `bufp` | `char **` | 指向调用者缓冲区指针的指针（输出参数，关闭时更新） |
| `sizep` | `size_t *` | 指向调用者大小变量的指针（输出参数，实时更新） |
| `pos` | `size_t` | 当前写入位置 |
| `buf` | `char *` | 内部分配的缓冲区指针 |
| `len` | `size_t` | 当前有效数据长度 |
| `space` | `size_t` | 缓冲区已分配总容量 |

### struct ms_FILE

```c
struct ms_FILE {
    FILE f;
    struct cookie c;
    unsigned char buf[BUFSIZ];
};
```

[Visibility]: Internal (不导出) — 动态内存流 `FILE` 对象，包含标准 `FILE` 结构体、cookie 状态、以及 `BUFSIZ` 字节的 FILE 缓冲区。

---

## 函数规约

### 1. open_memstream

```c
FILE *open_memstream(char **bufp, size_t *sizep);
```

[Visibility]: User — 声明于 `<stdio.h>`，POSIX.1-2008 标准接口，用户程序可直接调用。属于 GNU 扩展。

#### Intent

创建一个只写的动态内存流。写入的数据被动态分配到内存缓冲区中，该缓冲区根据需要自动增长。调用者通过 `bufp` 和 `sizep` 获取最终的缓冲区指针和大小。当流被 `fclose` 关闭时，缓冲区被终止为有效的 C 字符串，并且 `*bufp` 和 `*sizep` 被更新为最终状态。

#### 前置条件

- `bufp`: 指向 `char*` 的指针（非 `NULL`），将在此处返回最终缓冲区指针
- `sizep`: 指向 `size_t` 的指针（非 `NULL`），将在此处返回缓冲区大小（不包含 NULL 终止符）

#### 后置条件

- **Case 1: 成功** — 返回新创建的 `FILE*` 对象
  - 流已设置为只写模式（`F_NORD` 标志 — 不允许读取）
  - `fd` 设为 `-1`（无底层文件描述符）
  - `mode` 设为 `-1`（特殊值，表示内存流）
  - 流禁止行缓冲（`lbf = EOF`）
  - 初始缓冲区大小为 1 字节（`buf[0] = 0`，分配了空字符串）
  - `*sizep = 0`，`*bufp` 指向初始缓冲区
  - 流的 `write` 回调为 `ms_write`（自动动态增长）
  - 流的 `seek` 回调为 `ms_seek`（内存中 seek）
  - 流的 `close` 回调为 `ms_close`（不释放缓冲区，留给调用者）
  - 若未启用线程化（`!libc.threaded`），`f->lock = -1`
  - 通过 `__ofl_add` 注册到全局打开文件链表
- **Case 2: 失败** — 返回 `NULL`
  - 若初始内存分配失败（`malloc` 返回 `NULL`）
  - 注意：写入时的 `realloc` 失败不会导致 `open_memstream` 本身返回 `NULL`（流已成功打开，写入时可能返回短写入）

#### 系统算法

```
open_memstream(bufp, sizep):
  1. f = malloc(sizeof *f)
     if (!f) return NULL
  2. buf = malloc(sizeof *buf)          // 分配初始缓冲区（1 字节）
     if (!buf): free(f); return NULL
  3. memset(&f->f, 0, sizeof f->f)      // 清零 FILE 结构
  4. memset(&f->c, 0, sizeof f->c)      // 清零 cookie
  5. f->f.cookie = &f->c
  6. 初始化 cookie:
        c.bufp = bufp                    // 指向调用者的 buffer 指针
        c.sizep = sizep                  // 指向调用者的 size 指针
        c.pos = c.len = c.space = *sizep = 0
        c.buf = *bufp = buf              // 缓冲区引用
        *buf = 0                         // 初始空字符串
  7. 初始化 FILE:
        f->f.flags = F_NORD
        f->f.fd = -1
        f->f.buf = f->buf                // FILE 缓冲区
        f->f.buf_size = sizeof f->buf
        f->f.lbf = EOF
        f->f.write = ms_write
        f->f.seek = ms_seek
        f->f.close = ms_close
        f->f.mode = -1
  8. if (!libc.threaded) f->f.lock = -1
  9. return __ofl_add(&f->f)
```

#### 不变量

- `fd` 始终为 `-1`（无底层文件描述符）
- 流是只写的（通过 `F_NORD` 标志禁止读取）
- `*sizep` 始终反映当前缓冲区有效数据长度（不断更新，实时可见）
- 缓冲区始终以 NULL 终止（`buf[c.len] = 0` 或 `buf[0] = 0`）
- 关闭后 `*bufp` 指向的缓冲区归调用者所有，调用者负责 `free`
- 初始分配 `sizeof *buf` 字节（1 字节），然后自动增长

---

### 2. ms_write (static)

```c
static size_t ms_write(FILE *f, const unsigned char *buf, size_t len);
```

[Visibility]: Internal (不导出) — 动态内存流写入函数

#### Intent

将数据写入动态内存流缓冲区。先写出 FILE 自身缓冲区中的待写数据，然后将新数据写入 `cookie.buf`。若缓冲区空间不足，自动调用 `realloc` 以 2 倍（或 `pos+len+1`）的方式增长。

#### 前置条件

- `f` 指向有效的 `ms_FILE` 对象
- `buf` 指向有效数据

#### 后置条件

- **Case 1: 成功** — 数据已写入缓冲区，`c.pos` 和 `c.len` 已更新
  - `*sizep` 被更新为当前 `c.pos`（使调用者始终能读到最新大小）
  - 若 `realloc` 发生，`*bufp` 被更新为新缓冲区地址
  - 新增空间被零填充
- **Case 2: realloc 失败** — 返回 `0`（短写入）

#### 系统算法

```
ms_write(f, buf, len):
  c = f->cookie
  len2 = f->wpos - f->wbase            // FILE 写入缓冲区中已有数据
  if (len2):
    f->wpos = f->wbase                  // 重置 wpos
    if (ms_write(f, f->wbase, len2) < len2) return 0  // 递归先刷新

  if (len + c->pos >= c->space):        // 需要扩容
    len2 = 2 * c->space + 1 | c->pos + len + 1   // 计算新大小: max(2*s+1, pos+len+1)
    newbuf = realloc(c->buf, len2)
    if (!newbuf) return 0               // 分配失败: 短写入
    *c->bufp = c->buf = newbuf
    memset(c->buf + c->space, 0, len2 - c->space)  // 零填充新增空间
    c->space = len2

  memcpy(c->buf + c->pos, buf, len)
  c->pos += len
  if (c->pos >= c->len) c->len = c->pos // 更新有效长度
  *c->sizep = c->pos                    // 实时更新调用者大小
  return len
```

**注意**: 增长策略 `2*c->space+1 | c->pos+len+1` 确保在避免频繁 realloc 的同时，始终为新数据加上 NULL 终止符留足空间。

---

### 3. ms_seek (static)

```c
static off_t ms_seek(FILE *f, off_t off, int whence);
```

[Visibility]: Internal (不导出) — 动态内存流 seek 函数

#### Intent

在动态内存流中移动写入位置。支持 SEEK_SET/SEEK_CUR/SEEK_END。与 `fmemopen` 的 seek 不同，这里 `size` 理论上无限（动态增长），但当前位置不能超过当前缓冲区末端。

#### 前置条件

- `f` 指向有效的 `ms_FILE` 对象
- `whence` 为 `0`、`1` 或 `2`

#### 后置条件

- **Case 1: 成功** — 更新 `c->pos`，返回新位置
- **Case 2: 失败** — `errno = EINVAL`, 返回 `-1`
  - 若 `whence > 2`
  - 若 `off < -base` 或 `off > SSIZE_MAX - base`（越界）

#### 系统算法

```
ms_seek(f, off, whence):
  c = f->cookie
  if (whence > 2): errno=EINVAL, return -1
  base = (size_t[3]){0, c->pos, c->len}[whence]
  if (off < -base || off > SSIZE_MAX - base): errno=EINVAL, return -1
  return c->pos = base + off
```

---

### 4. ms_close (static)

```c
static int ms_close(FILE *f);
```

[Visibility]: Internal (不导出) — 动态内存流关闭函数

#### Intent

关闭操作。对 open_memstream 而言，该函数不执行实际清理工作（缓冲区所有权已转移给调用者）。始终成功返回 `0`。

#### 前置条件

- `f` 指向有效的 `ms_FILE` 对象

#### 后置条件

- **始终**: 返回 `0`
- 缓冲区不被释放（由调用者负责管理，通过 `*bufp` 返回）
