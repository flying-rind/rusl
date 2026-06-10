# fputwc.c 规约

> musl libc 宽字符单字符写入实现。将一个宽字符转换为多字节序列并写入 FILE 流。

---

## 依赖图

```
fputwc (Public)
  └── __fputwc_unlocked (hidden)
        ├── CURRENT_LOCALE (宏, 来自 locale_impl.h)
        ├── fwide (see fwide.c)
        ├── isascii (来自 <ctype.h>)
        ├── putc_unlocked (来自 stdio_impl.h)
        │     └── __overflow (see __overflow.c)
        ├── wctomb (来自 <wchar.h>)
        └── __fwritex (来自 stdio_impl.h, see fwrite.c)

fputwc_unlocked (weak_alias) ──> __fputwc_unlocked
putwc_unlocked (weak_alias) ──> __fputwc_unlocked
```

---

## 函数规约

### 1. __fputwc_unlocked

```c
wint_t __fputwc_unlocked(wchar_t c, FILE *f);
```

[Visibility]: Internal — `hidden` 可见性，不对外导出。通过 `fputwc_unlocked` 和 `putwc_unlocked` 弱别名间接暴露给用户

#### Intent

无锁宽字符写入接口。将宽字符 `c` 转换为多字节序列并写入 `f`。采用三级写入策略，按优先级选择最高效的路径：
1. ASCII 优化路径：若 `c` 是 ASCII 字符，直接委托 `putc_unlocked`
2. 宽缓冲区路径：若宽字符缓冲区有足够空间，直接写入并转换
3. 回退路径：通过临时缓冲区转换后批量写入

#### 前置条件

- `c`: 要写入的宽字符（`wchar_t` 类型，有效 Unicode 码点或 `WEOF`）
- `f`: 非空 FILE 指针，调用者可持有也可不持有锁
- 若 `f->mode <= 0`，内部调用 `fwide(f, 1)` 设置宽字符方向

#### 后置条件

- **Case 1 成功写入宽字符**
  - 返回写入的宽字符值 `c`
  - 宽字符已转换为多字节序列并写入流的写缓冲区

- **Case 2 写入错误或编码错误**
  - 返回 `WEOF`
  - `f->flags |= F_ERR`

#### 系统算法

```
__fputwc_unlocked(c, f):
  ploc = &CURRENT_LOCALE                   // 获取当前线程 locale 指针
  loc = *ploc                              // 保存当前 locale
  if (f->mode <= 0) fwide(f, 1)            // 确保宽字符方向
  *ploc = f->locale                        // 设置为流的 locale

  if (isascii(c)):                         // 路径 1: ASCII 快速路径
    c = putc_unlocked(c, f)                // 直接写入（不转换）
  else if (f->wpos + MB_LEN_MAX < f->wend):  // 路径 2: 宽字符缓冲区可用
    l = wctomb(f->wpos, c)                 // 直接在宽字符写指针处转换
    if (l < 0): c = WEOF                   // 编码错误
    else: f->wpos += l                     // 前进宽字符写指针
  else:                                    // 路径 3: 回退到临时缓冲区
    l = wctomb(mbc, c)                     // 在栈上转换为多字节
    if (l < 0 || __fwritex(mbc, l, f) < l)  // 批量写入
      c = WEOF                             // 编码错误或写入错误

  if (c == WEOF) f->flags |= F_ERR         // 设置错误标志
  *ploc = loc                              // 恢复当前 locale
  return c
```

#### 不变量

- 宽字符写入始终使用流的 locale 进行多字节转换
- 调用者的 locale 在返回时恢复

#### 依赖

- `CURRENT_LOCALE` — 当前线程 locale（来自 `locale_impl.h`）
- `fwide(FILE *, int)` — 流方向设置（见 `fwide.c`）
- `isascii(int)` — ASCII 字符判断（`<ctype.h>`）
- `putc_unlocked(int, FILE *)` — 无锁字节写入（来自 `stdio_impl.h`）
- `wctomb(char *, wchar_t)` — 宽字符到多字节转换（`<wchar.h>`）
- `__fwritex(void *, size_t, FILE *)` — 无锁缓冲批量写入（见 `fwrite.c`）
- `MB_LEN_MAX` — 多字节字符最大长度（`<limits.h>`）
- `F_ERR` — 文件错误标志（来自 `stdio_impl.h`）

---

### 2. fputwc

```c
wint_t fputwc(wchar_t c, FILE *f);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

将宽字符 `c` 写入 FILE 流 `f`。内部通过 `FLOCK` 加锁后委托 `__fputwc_unlocked`，返回前释放锁。

#### 前置条件

- `c`: 要写入的宽字符
- `f`: 非空 FILE 指针，指向已打开的写模式流

#### 后置条件

- **Case 1 成功写入宽字符**
  - 返回写入的宽字符值 `c`
  - 宽字符已转换为多字节序列并写入流

- **Case 2 写入错误或编码错误**
  - 返回 `WEOF`
  - FILE 流设置 `F_ERR` 标志

#### 系统算法

```
fputwc(c, f):
  FLOCK(f)                     // 获取流锁
  c = __fputwc_unlocked(c, f)  // 委托无锁版本
  FUNLOCK(f)                   // 释放流锁
  return c
```

#### 依赖

- `__fputwc_unlocked(wchar_t, FILE *)` — 同文件 hidden 函数
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（来自 `stdio_impl.h`）

---

### 3. fputwc_unlocked / putwc_unlocked (weak_alias)

```c
weak_alias(__fputwc_unlocked, fputwc_unlocked);
weak_alias(__fputwc_unlocked, putwc_unlocked);
```

[Visibility]: User — POSIX 免锁扩展，通过 `<wchar.h>` 对外导出

- **Intention**: 提供免锁版本的宽字符写入。与 `__fputwc_unlocked` 行为完全相同，直接复用其实现。
- 调用者自行负责流的锁管理。

前置/后置条件及行为：完全等同于 `__fputwc_unlocked`。
