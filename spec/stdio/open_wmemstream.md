# open_wmemstream.c 规约

> musl libc 宽字符动态内存流打开函数实现。创建一个自动增长的只写流，允许程序动态构建宽字符内存缓冲区，并通过输出参数获取最终缓冲区指针和大小。

---

## 依赖图

```
open_wmemstream (Public)
  ├─> wms_FILE (struct) — 内部定义
  ├─> cookie (struct) — 内部定义
  ├─> wms_seek (static) — seek 回调
  ├─> wms_write (static) — 写入回调
  │     ├─> mbsnrtowcs (来自 <wchar.h>)
  │     └─> realloc (来自 <stdlib.h>)
  ├─> wms_close (static) — 关闭回调
  ├─> __ofl_add (see ofl_add.c)
  ├─> fwide (see fwide.c)
  ├─> malloc / free (来自 <stdlib.h>)
  ├─> memset (来自 <string.h>)
  └─> libc.threaded (来自 libc.h)
```

---

## 内部类型定义

### struct cookie

```c
struct cookie {
    wchar_t **bufp;
    size_t *sizep;
    size_t pos;
    wchar_t *buf;
    size_t len;
    size_t space;
    mbstate_t mbs;
};
```

[Visibility]: Internal (不导出) — 宽字符动态内存流的内部状态控制块

| 字段 | 类型 | 含义 |
|------|------|------|
| `bufp` | `wchar_t **` | 指向调用者缓冲区指针的指针（输出参数） |
| `sizep` | `size_t *` | 指向调用者大小变量的指针（实时更新当前宽字符数） |
| `pos` | `size_t` | 当前写入位置（宽字符偏移） |
| `buf` | `wchar_t *` | 内部分配的宽字符缓冲区指针 |
| `len` | `size_t` | 当前有效数据长度（宽字符数） |
| `space` | `size_t` | 缓冲区已分配总容量（宽字符数） |
| `mbs` | `mbstate_t` | 多字节到宽字符的转换状态（支持跨调用增量转换） |

### struct wms_FILE

```c
struct wms_FILE {
    FILE f;
    struct cookie c;
    unsigned char buf[1];
};
```

[Visibility]: Internal (不导出) — 宽字符动态内存流 `FILE` 对象。包含标准 `FILE` 结构体、cookie 状态、以及 1 字节的 FILE 缓冲区（用于适配 `stdio` 缓冲框架，但不实际用于宽字符数据缓冲）。

---

## 函数规约

### 1. open_wmemstream

```c
FILE *open_wmemstream(wchar_t **bufp, size_t *sizep);
```

[Visibility]: User — 声明于 `<wchar.h>`，POSIX.1-2008 标准接口，用户程序可直接调用。

#### Intent

创建一个只写的宽字符动态内存流。写入的宽字符数据被动态分配到内存缓冲区中。调用者通过 `bufp` 和 `sizep` 获取最终的缓冲区指针和大小。当流被 `fclose` 关闭时，缓冲区被终止为有效的宽字符串，并且 `*bufp` 和 `*sizep` 被更新为最终状态。

与 `open_memstream` 的区别：
- 缓冲区存储宽字符（`wchar_t`）而非字节（`char`）
- 使用 `mbsnrtowcs` 进行多字节到宽字符的增量转换
- 缓冲区扩展以 4 字节为单位（`sizeof(wchar_t)` 通常为 4）

#### 前置条件

- `bufp`: 指向 `wchar_t*` 的指针（非 `NULL`），将在此处返回最终宽字符缓冲区指针
- `sizep`: 指向 `size_t` 的指针（非 `NULL`），将在此处返回宽字符数（不包含 `L'\0'` 终止符）

#### 后置条件

- **Case 1: 成功** — 返回新创建的 `FILE*` 对象
  - 流已设置为只写模式（`F_NORD` 标志）
  - `fd` 设为 `-1`
  - 初始宽字符缓冲区大小为 `sizeof(wchar_t)`（1 个宽字符 = `*buf = 0`）
  - `*sizep = 0`，`*bufp` 指向初始宽字符缓冲区
  - `fwide` 被调用设置为宽字符模式
  - 自定义回调：`wms_write`、`wms_seek`、`wms_close`
  - 若未启用线程化（`!libc.threaded`），`f->lock = -1`
  - 通过 `__ofl_add` 注册到全局打开文件链表
- **Case 2: 失败** — 返回 `NULL`
  - 若 `malloc` 分配 `f` 或初始 `buf` 失败

#### 系统算法

```
open_wmemstream(bufp, sizep):
  1. f = malloc(sizeof *f)               // 分配 wms_FILE
     if (!f) return NULL
  2. buf = malloc(sizeof *buf)           // 分配初始宽字符缓冲区 (1 个 wchar_t)
     if (!buf): free(f); return NULL
  3. memset(&f->f, 0, sizeof f->f)       // 清零 FILE 结构
  4. memset(&f->c, 0, sizeof f->c)       // 清零 cookie
  5. f->f.cookie = &f->c
  6. 初始化 cookie:
        c.bufp = bufp
        c.sizep = sizep
        c.pos = c.len = c.space = *sizep = 0
        c.buf = *bufp = buf
        *buf = 0                          // 初始空宽字符串
  7. 初始化 FILE:
        f->f.flags = F_NORD               // 只写
        f->f.fd = -1
        f->f.buf = f->buf                 // FILE 的窄字节缓冲区 (1 字节)
        f->f.buf_size = 0
        f->f.lbf = EOF
        f->f.write = wms_write
        f->f.seek = wms_seek
        f->f.close = wms_close
  8. if (!libc.threaded) f->f.lock = -1
  9. fwide(&f->f, 1)                      // 设置宽字符方向
  10. return __ofl_add(&f->f)
```

#### 不变量

- `fd` 始终为 `-1`
- 流是只写的（`F_NORD` 标志）
- `*sizep` 实时反映当前缓冲区宽字符数
- 缓冲区始终以 `L'\0'` 终止
- 关闭后 `*bufp` 归调用者所有，调用者负责 `free`
- 扩展以 `sizeof(wchar_t)` 为单位（`len2*4` 计算新容量）

---

### 2. wms_seek (static)

```c
static off_t wms_seek(FILE *f, off_t off, int whence);
```

[Visibility]: Internal (不导出) — 宽字符动态内存流 seek 函数

#### Intent

在宽字符动态内存流中移动写入位置。seek 时重置 `mbs` 转换状态（因为当前位置改变后之前的增量转换状态无效）。越界检查以防止 `c->pos` 溢出，限制最大位置为 `SSIZE_MAX / 4`（因为 `wchar_t` 为 4 字节）。

#### 前置条件

- `f` 指向有效的 `wms_FILE` 对象
- `whence` 为 `0`（`SEEK_SET`）、`1`（`SEEK_CUR`）或 `2`（`SEEK_END`）

#### 后置条件

- **Case 1: 合法 seek** — 更新 `c->pos`，重置 `c->mbs` 为零，返回新位置
- **Case 2: 非法 seek** — `errno = EINVAL`，返回 `-1`

#### 系统算法

```
wms_seek(f, off, whence):
  c = f->cookie
  if (whence > 2): goto fail
  base = {0, c->pos, c->len}[whence]
  if (off < -base || off > SSIZE_MAX/4 - base): goto fail
  memset(&c->mbs, 0, sizeof c->mbs)    // 重置转换状态
  return c->pos = base + off
fail:
  errno = EINVAL; return -1
```

---

### 3. wms_write (static)

```c
static size_t wms_write(FILE *f, const unsigned char *buf, size_t len);
```

[Visibility]: Internal (不导出) — 宽字符动态内存流写入函数

#### Intent

将 `vfwprintf` 产生的多字节输出通过 `mbsnrtowcs` 增量转换为宽字符并写入 cookie 的宽字符缓冲区。自动调用 `realloc` 以 `2*space+1` 方式增长缓冲区。与 `ms_write`（字节版）对称，但使用宽字符转换和宽字符缓冲区。

#### 前置条件

- `f` 指向有效的 `wms_FILE` 对象
- `buf` 指向有效多字节数据

#### 后置条件

- **Case 1: 成功** — 数据已转换为宽字符写入缓冲区
  - `c->pos`、`c->len` 已更新
  - `*sizep` 已更新为新 `c->pos`
- **Case 2: realloc 失败** — 返回 `0`
- **Case 3: 多字节转换错误** — 返回 `0`

#### 系统算法

```
wms_write(f, buf, len):
  c = f->cookie
  // 先递归刷出 FILE 自身写缓冲区中的待写数据
  len2 = f->wpos - f->wbase
  if (len2):
    f->wpos = f->wbase
    if (wms_write(f, f->wbase, len2) < len2): return 0

  // 检查是否需要扩容 (以宽字符为单位)
  if (len + c->pos >= c->space):
    len2 = 2 * c->space + 1 | c->pos + len + 1  // 新容量
    if (len2 > SSIZE_MAX / 4): return 0          // 溢出保护
    newbuf = realloc(c->buf, len2 * 4)            // *4 因为 sizeof(wchar_t)==4
    if (!newbuf): return 0
    *c->bufp = c->buf = newbuf
    memset(c->buf + c->space, 0, 4 * (len2 - c->space))
    c->space = len2

  // 增量多字节到宽字符转换
  len2 = mbsnrtowcs(c->buf + c->pos, &buf, len, c->space - c->pos, &c->mbs)
  if (len2 == (size_t)-1): return 0               // 转换错误
  c->pos += len2
  if (c->pos >= c->len): c->len = c->pos
  *c->sizep = c->pos                              // 实时更新调用者大小
  return len
```

#### 依赖

- `mbsnrtowcs()` — 有状态的多字节到宽字符串转换（`<wchar.h>`）
- `realloc()` — 动态内存重新分配（`<stdlib.h>`）
- `memset()` — 内存填充（`<string.h>`）
- `SSIZE_MAX` — `ssize_t` 最大值（`<limits.h>`）

---

### 4. wms_close (static)

```c
static int wms_close(FILE *f);
```

[Visibility]: Internal (不导出) — 宽字符动态内存流关闭函数

#### Intent

关闭操作。对 `open_wmemstream` 而言，此函数不执行实际清理（缓冲区所有权在关闭前已通过 `*bufp` 转移给调用者）。始终成功返回 `0`。

#### 后置条件

- **始终**: 返回 `0`
