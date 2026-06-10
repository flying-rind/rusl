# fputws.c 规约

> musl libc 宽字符串写入实现。将宽字符串转换为多字节序列并批量写入 FILE 流。

---

## 依赖图

```
fputws (Public)
  ├── CURRENT_LOCALE (宏, 来自 locale_impl.h)
  ├── fwide (see fwide.c)
  ├── wcsrtombs (来自 <wchar.h>)
  ├── __fwritex (see fwrite.c)
  └── FLOCK / FUNLOCK (来自 stdio_impl.h)

fputws_unlocked (weak_alias)
  └── fputws
```

---

## 函数规约

### 1. fputws

```c
int fputws(const wchar_t *restrict ws, FILE *restrict f);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

将宽字符串 `ws` 转换为多字节序列并写入 FILE 流 `f`。使用 `BUFSIZ` 大小的本地缓冲区进行批量转换，通过 `__fwritex` 每次写入一批转换后的字节。不写入终止 `L'\0'`。

#### 前置条件

- `ws`: 指向以 `L'\0'` 结尾的有效宽字符串；可以为 `NULL`（此时行为为无操作，返回 `0`）
- `f`: 非空 FILE 指针，指向已打开的写模式流

#### 后置条件

- **Case 1 成功写入完整字符串**
  - 返回 `0`
  - 所有宽字符已转换为多字节序列并写入流

- **Case 2 写入错误或编码错误**
  - 返回 `-1`
  - `f->flags` 可能设置 `F_ERR`

#### 系统算法

```
fputws(ws, f):
  buf[BUFSIZ]                           // 栈上转换缓冲区
  l = 0
  ploc = &CURRENT_LOCALE                // 获取并保存当前线程 locale
  loc = *ploc

  FLOCK(f)                              // 获取流锁
  fwide(f, 1)                           // 设置宽字符方向
  *ploc = f->locale                     // 设置为流的 locale

  while (ws && (l = wcsrtombs(buf, &ws, sizeof buf, 0)) + 1 > 1):
    // 转换一批宽字符到 buf，l 为转换后的字节数
    if (__fwritex(buf, l, f) < l):      // 批量写入
      FUNLOCK(f)
      *ploc = loc
      return -1

  FUNLOCK(f)                            // 释放流锁
  *ploc = loc                           // 恢复 locale
  return l                              // 0 表示成功；-1 表示 wcsrtombs 转换错误
```

**注意**: `wcsrtombs` 的循环条件 `l + 1 > 1` 等价于 `l != (size_t)-1`，因为 `(size_t)-1` 是 `wcsrtombs` 的错误返回值。

#### 依赖

- `CURRENT_LOCALE` — 当前线程 locale（来自 `locale_impl.h`）
- `fwide(FILE *, int)` — 流方向设置（见 `fwide.c`）
- `wcsrtombs(char *, const wchar_t **, size_t, mbstate_t *)` — 宽字符串到多字节字符串转换，支持状态和中间停止（`<wchar.h>`）
- `__fwritex(void *, size_t, FILE *)` — 无锁批量写入（见 `fwrite.c`）
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（来自 `stdio_impl.h`）
- `BUFSIZ` — 默认缓冲区大小宏（来自 `stdio_impl.h`）

---

### 2. fputws_unlocked (weak_alias)

```c
weak_alias(fputws, fputws_unlocked);
```

[Visibility]: User — POSIX 免锁扩展，通过 `<wchar.h>` 对外导出

- **Intention**: 提供免锁版本的宽字符串写入。在 musl 中 `fputws` 本身通过 `FLOCK` 加锁，`fputws_unlocked` 作为弱别名指向同一实现。实际行为与 `fputws` 相同。

前置/后置条件及行为：完全等同于 `fputws`。
