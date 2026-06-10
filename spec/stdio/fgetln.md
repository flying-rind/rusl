# fgetln.c 规约

> musl libc GNU 扩展行读取实现。`fgetln` 返回指向 FILE 流中一行数据的指针，避免拷贝，提供零分配的行读取方式。

---

## 依赖图

```
fgetln (Public)
  ├── FLOCK / FUNLOCK (锁宏)
  ├── getc_unlocked (宏, 定义于 stdio_impl.h)
  │     └── __uflow (see __uflow.c spec)
  ├── ungetc (from <stdio.h>)
  ├── memchr (from <string.h>)
  ├── getline (see getline.c spec)
  │     └── getdelim (see getdelim.c spec)
  └── f->getln_buf (FILE 内部缓冲区字段)
```

---

## 函数规约

### fgetln

```c
#define _GNU_SOURCE
char *fgetln(FILE *f, size_t *plen);
```

[Visibility]: User — GNU 扩展（`_GNU_SOURCE` 下可见），非 POSIX 标准函数，不保证可移植。用户包含 `<stdio.h>` 并定义 `_GNU_SOURCE` 即可调用

#### Intent

从 FILE 流 `f` 中返回指向下一行数据的指针（包括换行符），并通过 `*plen` 返回该行的长度。相比 `fgets` 和 `getline`：
- **零拷贝**: 若数据已在 FILE 读缓冲区中，直接返回指向缓冲区内部的指针，无需拷贝
- **零分配**: 仅当缓冲区中数据不足以找到完整行时，才通过 `getline` 动态分配内存到 `f->getln_buf`

注意返回值仅在下次对同一 FILE 流的 IO 操作之前有效。

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的读模式流
- `plen`: 非空指针，用于接收行长（包括换行符）

#### 后置条件

- **Case 1 数据已在缓冲区中包含完整行**
  - `*plen` 设置为该行长度（包括 `\n`）
  - 返回指向 `f->rpos` 原始位置的指针（在 FILE 缓冲区内部）
  - FILE 流位置前进到换行符之后
  - **注意**: 返回的指针指向 FILE 内部缓冲区，下次 IO 操作可能使其失效

- **Case 2 数据不在缓冲区中（或不够完整行），已到达 EOF**
  - 若 `getline` 返回 `> 0`：`*plen` 设置为行长，返回 `f->getln_buf`
  - 若 `getline` 返回 `<= 0`：返回 `NULL`，`*plen` 可能未被修改

- **Case 3 流无法读取或出错**
  - 返回 `NULL`

#### 系统算法

```
fgetln(f, plen):
  ret = NULL

  FLOCK(f)                                // 获取 FILE 锁

  // 步骤 1: 预读一个字符后将其推回
  // 目的: 强制将数据加载到 f->rend 缓冲区
  ungetc(getc_unlocked(f), f)

  // 步骤 2: 尝试从现有缓冲区中查找完整行
  if f->rend != NULL and (z = memchr(f->rpos, '\n', f->rend - f->rpos)):
    // 缓冲区中找到了 '\n'，返回指向缓冲区的零拷贝指针
    ret = (char *)f->rpos
    *plen = ++z - ret                     // 计算长度(含 '\n')
    f->rpos = (void *)z                   // 前进读指针
  else if (l = getline(&f->getln_buf, (size_t[]){0}, f)) > 0:
    // 缓冲区中未找到完整行，使用 getline 动态分配
    *plen = l                              // getline 返回的行长
    ret = f->getln_buf                     // 返回动态分配的缓冲区

  FUNLOCK(f)
  return ret
```

**`ungetc` 预读技巧**: `getc_unlocked(f)` 确保 FILE 的底层缓冲区被填充（若为空），随后立即 `ungetc` 推回该字符，保证流指针位置不变。此技巧确保 `f->rend` 指向有效数据，使后续 `memchr` 搜索有效。

**复合字面量初始化**: `(size_t[]){0}` 创建一个大小为 1 的匿名 size_t 数组，首元素为 0。传递给 `getline` 作为 `*n` 以强制其从零开始分配（即使 `f->getln_buf` 非空）。

#### 不变量

- FILE 锁在整个执行期间被持有
- 返回指针的有效期受限于下次对 `f` 的 IO 操作

#### 依赖

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁
- `getc_unlocked(f)` — 无锁字符读取宏
- `ungetc(int, FILE *)` — 字符推回（`<stdio.h>`）
- `memchr` — 内存搜索换行符（`<string.h>`）
- `getline(char **, size_t *, FILE *)` — 动态分配行读取（定义于 `getline.c`，委托给 `getdelim`）
- `f->getln_buf` — FILE 内部行缓冲区字段
