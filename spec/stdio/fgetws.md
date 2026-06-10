# fgetws.c 规约

> musl libc 宽字符行读取实现。从 FILE 流中读取一行宽字符串。

---

## 依赖图

```
fgetws (Public)
  ├── __fgetwc_unlocked (see fgetwc.c)
  ├── ferror (来自 <stdio.h>)
  └── FLOCK / FUNLOCK (来自 stdio_impl.h)

fgetws_unlocked (weak_alias)
  └── fgetws
```

---

## 函数规约

### 1. fgetws

```c
wchar_t *fgetws(wchar_t *restrict s, int n, FILE *restrict f);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从 FILE 流 `f` 中读取最多 `n-1` 个宽字符存入 `s`，遇到换行符 `L'\n'` 或 EOF 时停止。读取成功后以 `L'\0'` 终止字符串。若发生错误，`ferror(f)` 将返回非零且函数返回 `NULL`。

#### 前置条件

- `s`: 非空缓冲区指针，至少有 `n` 个 `wchar_t` 的存储空间
- `n`: 缓冲区大小（宽字符数），`n > 0`
- `f`: 非空 FILE 指针，指向已打开的读模式流
- 若 `n == 1`：不读取任何字符，写入 `L'\0'` 后直接返回 `s`

#### 后置条件

- **Case 1 成功读取（包括读到换行符）**
  - 返回 `s`
  - `s` 包含读取的宽字符并以 `L'\0'` 终止
  - 若读到换行符，换行符包含在结果中

- **Case 2 到达文件末尾但未读取任何字符**
  - 返回 `NULL`
  - `s` 内容不变

- **Case 3 读取过程中发生错误**
  - 返回 `NULL`
  - `ferror(f)` 返回非零

- **Case 4 读取过程中到达 EOF（但已读取了一些字符）**
  - 返回 `s`（与字节版 `fgets` 不同！`fgets` 此时也返回 `s`）
  - `s` 包含已读取的字符并以 `L'\0'` 终止

#### 系统算法

```
fgetws(s, n, f):
  p = s
  if (--n == 0):                    // n == 1 的特殊情况
    return s                        // 空字符串

  FLOCK(f)                          // 获取流锁

  for (; n > 0; n--):
    c = __fgetwc_unlocked(f)        // 逐宽字符读取
    if (c == WEOF): break           // EOF 或错误
    *p++ = c
    if (c == '\n'): break           // 换行符

  *p = 0                            // 终止字符串
  if (ferror(f)): p = s             // 发生错误，标记为返回 NULL

  FUNLOCK(f)                        // 释放流锁

  return (p == s) ? NULL : s        // 未读取到任何字符则返回 NULL
```

#### 不变量

- 始终以 `L'\0'` 终止 `s`（即使没有读取任何字符）
- 函数持有 `FLOCK(f)` 期间逐字符读取

#### 依赖

- `__fgetwc_unlocked(FILE *)` — 无锁宽字符读取（见 `fgetwc.c`）
- `ferror(FILE *)` — 检查流错误状态（`<stdio.h>`）
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（来自 `stdio_impl.h`）

---

### 2. fgetws_unlocked (weak_alias)

```c
weak_alias(fgetws, fgetws_unlocked);
```

[Visibility]: User — POSIX 免锁扩展，通过 `<wchar.h>` 对外导出

- **Intention**: 提供免锁版本的宽字符行读取。在 musl 中 `fgetws` 本身通过 `FLOCK` 加锁，`fgetws_unlocked` 作为弱别名指向同一实现。实际行为与 `fgetws` 相同。

前置/后置条件及行为：完全等同于 `fgetws`。
