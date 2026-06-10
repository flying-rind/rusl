# fwide.c 规约

> musl libc 流方向（orientation）设置/查询函数。设置或查询 FILE 流是宽字符模式还是字节模式。

---

## 依赖图

```
fwide (Public)
  ├─> FLOCK / FUNLOCK  (宏, 来自 stdio_impl.h)
  ├─> MB_CUR_MAX  (宏, 来自 <stdlib.h>)
  ├─> C_LOCALE  (宏, 来自 locale_impl.h)
  └─> UTF8_LOCALE  (宏, 来自 locale_impl.h)
```

---

## 函数规约

### 1. fwide

```c
int fwide(FILE *f, int mode);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

设置或查询 FILE 流的"方向"（orientation）。宽字符 I/O 函数（如 `fgetwc`、`fputwc`）和字节 I/O 函数（如 `fgetc`、`fputc`）不能在同一流上混用。`fwide` 允许：
1. 查询当前流方向（`mode == 0`）
2. 设置流方向为宽字符模式（`mode > 0`）
3. 设置流方向为字节模式（`mode < 0`）

方向一旦设置后不可更改（除非流重新打开）。

#### 前置条件

- `f`: 非空 FILE 指针，指向有效的 FILE 对象
- 流方向尚未固定，或调用仅为查询（`mode == 0`）

#### 后置条件

- 返回值：流当前的方向
  - `> 0`：宽字符模式
  - `< 0`：字节模式
  - `== 0`：方向尚未设置（新打开的流）
- 若 `mode != 0` 且流方向尚未固定：
  - `f->mode` 被设置为 `mode > 0 ? 1 : -1`
  - `f->locale` 被初始化为合适的 locale（`MB_CUR_MAX == 1` 时使用 `C_LOCALE`，否则使用 `UTF8_LOCALE`）
- 若 `mode == 0` 或方向已固定：不修改任何状态，仅查询
- 函数持有 `FLOCK(f)` 锁期间执行，返回前释放

#### 系统算法

```
fwide(f, mode):
  FLOCK(f)                         // 获取流锁
  if (mode != 0):                  // 尝试设置方向
    if (!f->locale):               // 首次设置 locale
      f->locale = MB_CUR_MAX == 1
        ? C_LOCALE                // 单字节 locale，使用 C locale
        : UTF8_LOCALE             // 多字节 locale，使用 UTF-8 locale
    if (!f->mode):                 // 方向尚未固定
      f->mode = mode > 0 ? 1 : -1 // 设置方向
  mode = f->mode                   // 读取当前方向
  FUNLOCK(f)                       // 释放流锁
  return mode                      // 返回当前方向
```

#### 不变量

- `f->mode` 一旦设置为非零值后永不改变（除非流关闭后重新打开）
- 方向为 `1`（宽字符）或 `-1`（字节），不会有 `0` 以外的歧义值
- 首次非零设置时同步初始化 `f->locale`

#### 依赖

- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（见 `src/internal/stdio_impl.h`）
- `MB_CUR_MAX` — 当前 locale 的最大多字节字符长度（`<stdlib.h>`）
- `C_LOCALE` — C locale 常量（见 `src/internal/locale_impl.h`）
- `UTF8_LOCALE` — UTF-8 locale 常量（见 `src/internal/locale_impl.h`）
