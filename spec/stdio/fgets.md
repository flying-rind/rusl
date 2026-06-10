# fgets.c 规约

> musl libc 标准 IO 行读取实现。从 FILE 流中读取一行字符串（包括换行符）到用户缓冲区。

---

## 依赖图

```
fgets (Public)
  ├── FLOCK / FUNLOCK (锁宏)
  ├── memchr (from <string.h>)
  ├── memcpy (from <string.h>)
  ├── getc_unlocked (宏, 定义于 stdio_impl.h)
  │     └── __uflow (see __uflow.c spec)
  ├── feof (宏, 定义于 stdio_impl.h)
  └── weak_alias → fgets_unlocked
```

---

## 内部宏定义

### MIN

```c
#define MIN(a,b) ((a)<(b) ? (a) : (b))
```

[Visibility]: Internal — 文件内部宏

---

## 函数规约

### 1. fgets

```c
char *fgets(char *restrict s, int n, FILE *restrict f);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

从 FILE 流 `f` 中读取至多 `n-1` 个字符到缓冲区 `s`，遇到换行符 `\n` 或文件末尾时停止。读取的字符串总是以 `\0` 结尾（`n>=1` 时）。换行符会被保留在缓冲区中（若遇到）。

实现采用两阶段策略：
1. **批量拷贝**: 先利用 FILE 内部读缓冲区，用 `memchr` 快速定位 `\n`，用 `memcpy` 批量拷贝
2. **逐字符读取**: 缓冲区耗尽后逐字符读取，效率较低但确保正确性

#### 前置条件

- `s`: 非空指针，指向至少 `n` 字节的有效可写内存
- `n`: 缓冲区大小（`int`），至少为 1（否则无操作返回 NULL）
- `f`: 非空 FILE 指针，指向已打开的读模式流

#### 后置条件

- **Case 1 成功读取一行（包括换行符）**
  - 返回 `s`（指向用户缓冲区）
  - `s` 中以 `\0` 结尾，包含 `\n` 换行符（若在 `n-1` 字符内遇到）
  - 实际读取字符数 `<= n-1`

- **Case 2 成功读取但未遇到换行（缓冲区满）**
  - 返回 `s`
  - `s` 中包含 `n-1` 个字符 + `\0`
  - 下一次 `fgets` 调用将继续读取后续字符

- **Case 3 到达文件末尾且未读取任何字符**
  - 返回 `NULL`
  - `s` 内容不变
  - FILE 流设置 `F_EOF` 标志

- **Case 4 `n < 1`**
  - 返回 `NULL`，不执行任何操作

- **Case 5 读取错误（且未读取到任何字符）**
  - 返回 `NULL`

- **Case 6 `n == 1`**
  - 返回 `s`，仅写入 `\0`（空字符串）

#### 系统算法

```
fgets(s, n, f):
  p = s

  FLOCK(f)                                 // 获取 FILE 锁

  // 边界处理: n <= 1
  if n <= 1:
    f->mode |= f->mode-1                   // 设置读模式
    FUNLOCK(f)
    if n < 1: return NULL
    *s = '\0'
    return s

  n--                                      // 为 '\0' 保留一个位置

  // 主循环: 读取字符直到遇到 '\n' 或 n 耗尽
  while n > 0:
    // 阶段 1: 尽可能从 FILE 缓冲区批量拷贝
    if f->rpos != f->rend:
      z = memchr(f->rpos, '\n', f->rend - f->rpos)   // 搜索换行位置
      k = z ? z - f->rpos + 1 : f->rend - f->rpos     // 可拷贝字节数(含换行)
      k = MIN(k, n)                                    // 不超过剩余空间
      memcpy(p, f->rpos, k)
      f->rpos += k
      p += k
      n -= k
      if z != NULL or n == 0: break                    // 找到换行或缓冲区满

    // 阶段 2: 逐字符读取(缓冲区已空)
    c = getc_unlocked(f)                     // 读取一个字符
    if c < 0:                                // EOF 或错误
      if p == s or !feof(f): s = NULL        // 未读取任何字符则返回 NULL
      break
    n--
    *p++ = c
    if c == '\n': break                      // 遇到换行即停止

  // 终止字符串
  if s != NULL: *p = '\0'

  FUNLOCK(f)
  return s
```

#### 不变量

- 循环中 `p - s` 始终等于已存入缓冲区的字符数
- `n` 始终等于剩余可写入字符数（不含结尾 `\0`）
- FILE 锁在整个执行期间被持有

#### 依赖

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁
- `memchr` — 在缓冲区中搜索换行符（`<string.h>`）
- `memcpy` — 批量拷贝（`<string.h>`）
- `getc_unlocked(f)` — 无锁单字符读取宏（定义于 `stdio_impl.h`）
- `feof(f)` — 检测 EOF 标志宏（定义于 `stdio_impl.h`）

---

### 2. fgets_unlocked (weak_alias)

```c
// weak_alias(fgets, fgets_unlocked);
char *fgets_unlocked(char *restrict s, int n, FILE *restrict f);
```

[Visibility]: User — POSIX 免锁 `fgets`，在 musl 中与 `fgets` 共享同一实现

前置/后置条件及行为：完全等同于 `fgets`。
