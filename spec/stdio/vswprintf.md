# vswprintf.c 规约

> musl libc 宽字符串格式化输出函数（va_list 版本）。创建自定义只写 FILE 流，将格式化宽字符输出写入用户提供的宽字符缓冲区。

---

## 依赖图

```
vswprintf (Public)
  ├─> cookie (struct) — 内部状态
  ├─> sw_write (static) — 写入回调
  │     ├─> mbtowc (来自 <wchar.h>) — 多字节到宽字符转换
  │     └─> f->wbase / f->wpos / f->wend — FILE 缓冲区管理
  └─> vfwprintf (see vfwprintf.c) — 格式化引擎

swprintf (Public)
  └─> vswprintf
```

---

## 内部类型定义

### struct cookie

```c
struct cookie {
    wchar_t *ws;
    size_t l;
};
```

[Visibility]: Internal (不导出) — vswprintf 内部状态控制块

| 字段 | 类型 | 含义 |
|------|------|------|
| `ws` | `wchar_t *` | 指向用户提供目标缓冲区的当前写入位置指针 |
| `l` | `size_t` | 剩余可写入的宽字符数（含 `L'\0'` 终止符） |

---

## 函数规约

### 1. sw_write (static)

```c
static size_t sw_write(FILE *f, const unsigned char *s, size_t l);
```

[Visibility]: Internal (不导出) — 自定义 FILE 写入回调

#### Intent

将 `vfwprintf` 产生的多字节输出转换为宽字符并写入用户提供的宽字符缓冲区 `cookie.ws`。采用递归刷出策略：先递归处理 FILE 自身写缓冲区中的待写数据，再将新数据逐批转换。

#### 前置条件

- `f` 指向有效的自定义 FILE 对象（由 `vswprintf` 在栈上创建）
- `s` 指向多字节数据
- `cookie.ws` 指向用户缓冲区的当前位置

#### 后置条件

- **Case 1 成功** — 返回 `l0`（输入长度），数据已转换为宽字符写入 `cookie.ws`
  - `cookie.ws` 已更新为下一个写入位置
  - `cookie.l` 已减少相应数量
  - `*cookie.ws` 已被设为 `L'\0'` 终止符
- **Case 2 多字节转换错误 (`mbtowc` 返回 `-1`)**
  - 返回 `i`（错误值）
  - `f->wpos = f->wbase = f->wend = 0`（清零写指针）
  - `f->flags |= F_ERR`

#### 系统算法

```
sw_write(f, s, l):
  c = f->cookie
  l0 = l  // 保存原始长度

  // 先递归刷出 FILE 自身缓冲区中的待写数据
  if (s != f->wbase):
    if (sw_write(f, f->wbase, f->wpos - f->wbase) == -1): return -1

  // 逐字符将多字节数据转换为宽字符
  i = 0
  while (c->l > 0 && l > 0):
    // mbtowc: 返回 >0=消费字节数, 0=空字符(消费1字节), -1=错误
    i = mbtowc(c->ws, s, l)
    if (i >= 0):
      if (i == 0): i = 1         // 空字符视为 1 字节
      s += i; l -= i
      c->l--; c->ws++            // 前进
    else:
      // 转换错误
      f->wpos = f->wbase = f->wend = 0
      f->flags |= F_ERR
      return i

  *c->ws = 0                     // 终止符
  f->wend = f->buf + f->buf_size // 重置 FILE 写缓冲区指针
  f->wpos = f->wbase = f->buf
  return l0
```

#### 依赖

- `mbtowc()` — 多字节到宽字符转换（`<wchar.h>`）
- `F_ERR` — 文件错误标志（来自 `stdio_impl.h`）

---

### 2. vswprintf

```c
int vswprintf(wchar_t *restrict s, size_t n, const wchar_t *restrict fmt, va_list ap);
```

[Visibility]: User — `<stdarg.h>` / `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

将格式化宽字符串输出到缓冲区 `s`，最多写入 `n` 个宽字符（含终止 `L'\0'`）。通过创建自定义 FILE 对象并设置 `sw_write` 为写入回调，将 `vfwprintf` 的输出重定向到用户提供的宽字符缓冲区。

与 C99 标准的区别：musl 实现中，当输出被截断（`ret >= n`）时返回 `-1` 而非截断前的完整长度。

#### 前置条件

- `s`: 指向有效宽字符缓冲区的指针（`n > 0` 时）；`n == 0` 时可为 `NULL`
- `n`: 缓冲区大小（宽字符数）
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功（输出未截断）：返回写入的宽字符数（不含 `L'\0'`）
- Case 2 截断（`ret >= n`）：返回 `-1`
- Case 3 `n == 0`：返回 `-1`（尚未调用 `vfwprintf` 即返回）
- Case 4 输出错误/格式错误/溢出：返回 `-1`
- `s` 始终被 `L'\0'` 终止（当 `n > 0` 时）

#### 系统算法

```
vswprintf(s, n, fmt, ap):
  if (n == 0): return -1

  // 创建自定义 FILE 对象
  buf[256]              // 栈上 FILE 缓冲区
  cookie = { s, n - 1 } // 初始化 cookie，n-1 为终止符预留空间
  FILE f = {
    .lbf = EOF,         // 禁用行缓冲
    .write = sw_write,  // 自定义写入回调
    .lock = -1,         // 免锁 FILE
    .buf = buf,
    .buf_size = sizeof buf,
    .cookie = &c,
  }

  r = vfwprintf(&f, fmt, ap)  // 委托宽字符格式化引擎
  sw_write(&f, 0, 0)          // 确保最终刷出

  return (r >= n) ? -1 : r    // 截断检测
```

#### 不变量

- 缓冲区始终以 `L'\0'` 终止
- 自定义 FILE 为免锁模式（`lock = -1`），因为不涉及多线程
- 自定义 FILE 不进行行缓冲（`lbf = EOF`，始终全缓冲）

#### 依赖

- `vfwprintf()` — 宽字符格式化输出核心引擎（见 `vfwprintf.c`）
- `sw_write()` (static) — 自定义写入回调（同文件）
- `mbtowc()` — 多字节到宽字符转换（`<wchar.h>`）
- `EOF` — 文件结束标志（`<stdio.h>`）
