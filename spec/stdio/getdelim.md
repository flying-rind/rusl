# getdelim.c 规约

> musl libc 带分隔符的动态行读取实现。`getdelim` 从 FILE 流中读取以指定分隔符（或 EOF）结尾的一行，自动扩展或分配输出缓冲区。

---

## 依赖图

```
getdelim (Public)
  ├── FLOCK / FUNLOCK (锁宏)
  ├── memchr (from <string.h>)
  ├── memcpy (from <string.h>)
  ├── realloc (from <stdlib.h>)
  ├── getc_unlocked (宏, 定义于 stdio_impl.h)
  │     └── __uflow (see __uflow.c spec)
  ├── feof (宏, 定义于 stdio_impl.h)
  ├── errno / EINVAL / ENOMEM (from <errno.h>)
  └── weak_alias → __getdelim
```

---

## 函数规约

### 1. getdelim

```c
ssize_t getdelim(char **restrict s, size_t *restrict n, int delim, FILE *restrict f);
```

[Visibility]: User — POSIX.1-2008 标准函数，用户程序包含 `<stdio.h>` 即可调用

#### Intent

从 FILE 流 `f` 中读取以字符 `delim` 分隔（或 EOF 结尾）的一行数据到动态分配的缓冲区 `*s`。与 `fgets` 相比：
- **自动分配**: 若 `*s` 为 NULL 或 `*n` 不足以容纳完整行，自动通过 `realloc` 扩展
- **分隔符灵活**: 可指定任意字符作为行分隔符（不限于 `\n`）
- **保留分隔符**: 读取内容包含分隔符（除 EOF 结尾外）

典型用法是将 `*s` 初始化为 NULL，让 `getdelim` 自动分配；或提供预分配缓冲区由 `getdelim` 按需扩展。

#### 前置条件

- `s`: 非空指针，指向 `char *` 变量（该变量可为 NULL 或指向已有缓冲区）
- `n`: 非空指针，指向 `size_t` 变量（指示当前缓冲区大小，`*s` 为 NULL 时自动设为 0）
- `delim`: 分隔符字符（以 `int` 传递，内部转为 `unsigned char` 比较）
- `f`: 非空 FILE 指针，指向已打开的读模式流

#### 后置条件

- **Case 1 成功读取一行（以分隔符结尾）**
  - 返回读取的字符数（**包括**分隔符字符，不包括结尾的 `\0`）
  - `*s` 指向包含完整行 + `\0` 的动态分配缓冲区
  - `*n` 更新为当前缓冲区大小
  - `*s` 中第 `返回值` 字节为分隔符，第 `返回值 + 1` 字节为 `\0`

- **Case 2 成功读取到文件末尾（无分隔符）**
  - 返回读取的字符数（不包括 `\0`，**不包括**分隔符）
  - `*s` 指向包含数据 + `\0` 的缓冲区
  - FILE 流设置 `F_EOF` 标志

- **Case 3 参数无效**
  - `s` 或 `n` 为 NULL：返回 -1，`errno = EINVAL`，FILE 设置 `F_ERR`
  - 此情况下 `*s` 和 `*n` 保持不变

- **Case 4 内存不足（realloc 失败）**
  - 返回 -1，`errno = ENOMEM`，FILE 设置 `F_ERR`
  - 在返回前尽可能将已读取数据拷贝到现有缓冲区（截断适配）
  - `f->rpos` 位置已前进（缓冲区数据已被消费）

- **Case 5 读取错误（且未读取到任何字符）**
  - 返回 -1

#### 系统算法

```
getdelim(s, n, delim, f):
  i = 0                                    // 已读取字符计数

  FLOCK(f)                                 // 获取 FILE 锁

  // 参数校验
  if n == NULL or s == NULL:
    f->mode |= f->mode-1
    f->flags |= F_ERR
    FUNLOCK(f)
    errno = EINVAL
    return -1

  // 初始分配
  if *s == NULL: *n = 0

  // 主循环
  loop:
    // 步骤 1: 从 FILE 缓冲区搜索分隔符
    if f->rpos != f->rend:
      z = memchr(f->rpos, delim, f->rend - f->rpos)
      k = z ? z - f->rpos + 1 : f->rend - f->rpos   // 可拷贝字节数
    else:
      z = NULL; k = 0

    // 步骤 2: 检验输出缓冲是否足够，不足则扩展
    if i + k >= *n:
      m = i + k + 2
      if not z and m < SIZE_MAX/4: m += m/2     // 几何增长策略
      tmp = realloc(*s, m)
      if tmp == NULL:
        // 极端情况: 尝试最小分配
        m = i + k + 2
        tmp = realloc(*s, m)
        if tmp == NULL:
          // 分配失败: 尽可能复制已有数据
          k = *n - i
          memcpy(*s + i, f->rpos, k)
          f->rpos += k
          f->mode |= f->mode-1
          f->flags |= F_ERR
          FUNLOCK(f)
          errno = ENOMEM
          return -1
      *s = tmp; *n = m

    // 步骤 3: 批量拷贝可用数据
    if k > 0:
      memcpy(*s + i, f->rpos, k)
      f->rpos += k
      i += k

    // 步骤 4: 找到分隔符则终止
    if z != NULL: break

    // 步骤 5: 逐字符读取
    c = getc_unlocked(f)
    if c == EOF:
      if i == 0 or !feof(f):               // 未读任何字符或非 EOF 导致的错误
        FUNLOCK(f)
        return -1
      break                                  // EOF 且已读取数据，接受当前内容
    // 若下一个字符无法放入当前缓冲，将其推回
    if i + 1 >= *n: *--f->rpos = c          // 推回字符
    else if ((*s)[i++] = c) == delim: break  // 存入并检查分隔符
  endloop

  // 终止字符串
  (*s)[i] = '\0'

  FUNLOCK(f)
  return i
```

**缓冲区增长策略**:
- 首次分配或空间不足时，目标大小 `m = i + k + 2`（当前内容 + 可用内容 + 分隔符 + `\0`）
- 若未找到分隔符 (`!z`) 且 `m < SIZE_MAX/4`，采用几何增长: `m += m/2`（增长 50%），减少 `realloc` 调用次数
- 若几何增长后的 `realloc` 失败，尝试最小分配再失败才报错

**字符推回技巧**: 当 `getc_unlocked` 读取的字符无法放入输出缓冲区时，通过 `*--f->rpos = c` 将字符重新放回 FILE 读缓冲区，下一轮循环的步骤 1 会再次读取该字符。这避免了丢失字符。

**内存耗尽时的尽力而为**: 即使 `realloc` 失败，函数也会尽力将 FILE 缓冲区中的现有数据拷贝到输出缓冲区（截断适配），确保用户缓冲区中至少包含已读取的数据。

#### 不变量

- `i` 始终等于已存入 `*s` 的字符数（不含 `\0`）
- `*s` 始终指向至少 `*n` 字节的有效可用缓冲区（NULL 时除外，但会被自动分配）
- FILE 锁在整个执行期间被持有（除非早期返回错误）

#### 依赖

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁
- `memchr` — 搜索分隔符（`<string.h>`）
- `memcpy` — 批量拷贝（`<string.h>`）
- `realloc` — 动态内存重分配（`<stdlib.h>`）
- `getc_unlocked(f)` — 无锁字符读取宏
- `feof(f)` — 检测 EOF 标志
- `errno`, `EINVAL`, `ENOMEM` — 错误报告（`<errno.h>`）
- `SIZE_MAX` — 防止溢出（`<inttypes.h>` 或 `<stdint.h>`）

---

### 2. __getdelim (weak_alias)

```c
// weak_alias(getdelim, __getdelim);
ssize_t __getdelim(char **restrict s, size_t *restrict n, int delim, FILE *restrict f);
```

[Visibility]: Internal — `__` 前缀别名，供 musl 内部直接调用（如 `getline` 调用 `getdelim` 而非 `__getdelim`；但 musl 内部可能通过此符号绕过宏定义）

前置/后置条件及行为：完全等同于 `getdelim`。
