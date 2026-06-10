# fgetwc.c 规约

> musl libc 宽字符单字符读取实现。从 FILE 流中读取一个宽字符，处理多字节到宽字符的转换。

---

## 依赖图

```
fgetwc (Public)
  └── __fgetwc_unlocked (hidden)
        ├── CURRENT_LOCALE (宏, 来自 locale_impl.h)
        ├── fwide (see fwide.c)
        └── __fgetwc_unlocked_internal (static)
              ├── mbtowc (来自 <wchar.h>, 内部多字节转换)
              ├── mbrtowc (来自 <wchar.h>, 多字节转换 + 状态)
              ├── getc_unlocked (来自 stdio_impl.h)
              │     └── __uflow (see __uflow.c)
              └── ungetc (来自 <stdio.h>)

fgetwc_unlocked (weak_alias) ──> __fgetwc_unlocked
getwc_unlocked (weak_alias) ──> __fgetwc_unlocked
```

---

## 函数规约

### 1. __fgetwc_unlocked_internal (static)

```c
static wint_t __fgetwc_unlocked_internal(FILE *f);
```

[Visibility]: Internal — `static` 函数，不对外导出

#### Intent

无锁宽字符读取核心引擎。负责将 FILE 流读缓冲区中的多字节序列转换为单个宽字符。处理两阶段：
1. 优化路径：若读缓冲区有数据，直接用 `mbtowc` 尝试转换
2. 逐字节路径：若缓冲区不包含完整多字节字符，逐字节通过 `getc_unlocked` 读取并用 `mbrtowc`（带状态）累计转换

#### 前置条件

- `f`: 非空 FILE 指针，调用者已持有 `f` 的锁（或流为免锁模式）
- 流的 locale 已正确设置

#### 后置条件

- **Case 1 成功转换宽字符**
  - 返回转换后的 `wchar_t` 值
  - `f->rpos` 前进已消费的字节数

- **Case 2 到达文件末尾（首字节即 EOF）**
  - 返回 `WEOF`
  - 不设置 `F_ERR` 和 `errno`

- **Case 3 编码错误（非首字节的 EOF 或无效序列）**
  - 返回 `WEOF`
  - `f->flags |= F_ERR`，`errno = EILSEQ`
  - 若有多余字节，调用 `ungetc` 将其推回

#### 系统算法

```
__fgetwc_unlocked_internal(f):
  // Phase 1: 从缓冲区直接转换
  if (f->rpos != f->rend):                   // 读缓冲区有数据
    l = mbtowc(&wc, f->rpos, f->rend - f->rpos)
    if (l + 1 >= 1):                          // l >= 0 (成功转换) 或 l == -2 (不完整序列)
      f->rpos += l + (l == 0 ? 1 : 0)        // l==0 表示空字符，消费 1 字节
      return wc                               // 注意：l == -2 时 wc 未定义，但位运算后直接返回

  // Phase 2: 逐字节读取并转换
  mbstate_t st = {0}                          // 初始化转换状态
  first = 1
  do:
    b = c = getc_unlocked(f)                  // 获取下一个字节
    if (c < 0):                               // EOF
      if (!first):                            // 非首字节的 EOF 是编码错误
        f->flags |= F_ERR
        errno = EILSEQ
      return WEOF
    l = mbrtowc(&wc, &b, 1, &st)             // 带状态转换
    if (l == -1):                             // 无效序列
      if (!first):
        f->flags |= F_ERR
        ungetc(b, f)                          // 推回无效字节
      return WEOF
    first = 0
  while (l == -2)                             // 不完整序列，继续读取

  return wc
```

#### 依赖

- `mbtowc()` — 无状态多字节到宽字符转换（`<wchar.h>`）
- `mbrtowc()` — 有状态多字节到宽字符转换（`<wchar.h>`）
- `getc_unlocked` — 无锁字节读取（来自 `stdio_impl.h`）
- `ungetc()` — 推回字符（`<stdio.h>`）
- `F_ERR` — 文件错误标志（来自 `stdio_impl.h`）
- `EILSEQ` — 非法字节序列错误码（`<errno.h>`）

---

### 2. __fgetwc_unlocked

```c
wint_t __fgetwc_unlocked(FILE *f);
```

[Visibility]: Internal — `hidden` 可见性，不对外导出。通过 `fgetwc_unlocked` 和 `getwc_unlocked` 弱别名间接暴露给用户

#### Intent

无锁宽字符读取接口。保存/恢复当前线程 locale，设置为流的 locale 后调用 `__fgetwc_unlocked_internal`。确保宽字符转换使用正确的 locale。

#### 前置条件

- `f`: 非空 FILE 指针，调用者可持有也可不持有锁（此函数不负责加锁）
- 若 `f->mode <= 0`（方向未设置或为字节模式），内部调用 `fwide(f, 1)` 设置宽字符方向

#### 后置条件

- 返回值和错误处理同 `__fgetwc_unlocked_internal`
- 调用者的 locale 在返回时恢复

#### 系统算法

```
__fgetwc_unlocked(f):
  ploc = &CURRENT_LOCALE              // 获取当前线程 locale 指针
  loc = *ploc                         // 保存当前 locale
  if (f->mode <= 0) fwide(f, 1)       // 确保宽字符方向
  *ploc = f->locale                   // 设置为流的 locale
  wc = __fgetwc_unlocked_internal(f)  // 执行读取
  *ploc = loc                         // 恢复当前 locale
  return wc
```

#### 依赖

- `CURRENT_LOCALE` — 当前线程 locale（来自 `locale_impl.h`）
- `fwide(FILE *, int)` — 流方向设置（见 `fwide.c`）
- `__fgetwc_unlocked_internal(FILE *)` — 同文件 static 核心引擎

---

### 3. fgetwc

```c
wint_t fgetwc(FILE *f);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从 FILE 流 `f` 中读取一个宽字符。内部通过 `FLOCK` 加锁后委托 `__fgetwc_unlocked`，返回前释放锁。

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的流

#### 后置条件

- **Case 1 成功读取宽字符**
  - 返回读取到的宽字符（`wchar_t` 类型的 `wint_t` 值）
  - FILE 流位置前进

- **Case 2 到达文件末尾**
  - 返回 `WEOF`
  - FILE 流设置 `F_EOF` 标志

- **Case 3 读取错误或编码错误**
  - 返回 `WEOF`
  - FILE 流设置 `F_ERR` 标志
  - 编码错误时设置 `errno = EILSEQ`

#### 系统算法

```
fgetwc(f):
  FLOCK(f)                            // 获取流锁
  c = __fgetwc_unlocked(f)            // 委托无锁版本
  FUNLOCK(f)                          // 释放流锁
  return c
```

#### 依赖

- `__fgetwc_unlocked(FILE *)` — 同文件 hidden 函数
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（来自 `stdio_impl.h`）

---

### 4. fgetwc_unlocked / getwc_unlocked (weak_alias)

```c
weak_alias(__fgetwc_unlocked, fgetwc_unlocked);
weak_alias(__fgetwc_unlocked, getwc_unlocked);
```

[Visibility]: User — POSIX 免锁扩展，通过 `<wchar.h>` 对外导出

- **Intention**: 提供免锁版本的宽字符读取。与 `__fgetwc_unlocked` 行为完全相同，直接复用其实现。
- 调用者自行负责流的锁管理。

前置/后置条件及行为：完全等同于 `__fgetwc_unlocked`。
