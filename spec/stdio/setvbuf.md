# setvbuf.c 规约

> musl libc `setvbuf` 实现 — 所有缓冲设置函数的最终实现。直接操作 FILE 结构体的内部字段，不执行加锁（因为标准规定 setvbuf 仅在流打开后首次操作前调用时行为才定义良好）。

---

## 依赖图

```
setvbuf (Public)
  └── (无内部调用 — 仅直接操作 FILE 结构体字段)
```

---

## 内部类型 / 结构体字段

`setvbuf` 直接访问 `struct _IO_FILE` 中的以下字段（定义于 `src/internal/stdio_impl.h`）：

| 字段 | 类型 | 用途 |
|------|------|------|
| `f->lbf` | `int` | 行缓冲标志。`EOF`（-1）= 全缓冲/无缓冲；`'\n'` = 行缓冲 |
| `f->buf` | `unsigned char *` | 流内部缓冲区指针（偏移 UNGET 后的实际起始位置） |
| `f->buf_size` | `size_t` | 有效缓冲区大小（已扣除 UNGET 预留空间） |
| `f->flags` | `unsigned` | 文件流状态标志位集 |

---

## 函数规约

### 1. setvbuf

```c
int setvbuf(FILE *restrict f, char *restrict buf, int type, size_t size);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

设置 FILE 流的缓冲模式、缓冲区位置和大小。这是 musl 中 `setbuf`、`setbuffer`、`setlinebuf` 的底层实现。

**关键设计决策**：不执行任何 FILE 锁操作。理由由源码注释给出——该函数的行为由标准定义为仅在流上首次操作前调用才可预测，因此在一个行为定义良好的程序中，加锁的存在与否是不可观察的。

**缓冲区布局**：若调用方提供了缓冲区且 `size >= UNGET`（8），musl 在内部分配 `UNGET` 字节预留空间用于字符回退操作。实际缓冲区起始于 `buf + UNGET`，有效大小为 `size - UNGET`。若 `size < UNGET`，则不使用调用方提供的缓冲区。

#### 前置条件

- `f`: 有效的 `FILE *` 指针，指向已打开的流
- `buf`: 调用方提供的缓冲区（可为 NULL）
- `type`: 缓冲类型，必须为以下值之一：
  - `_IOFBF` (0): 全缓冲
  - `_IOLBF` (1): 行缓冲
  - `_IONBF` (2): 无缓冲
- `size`: 缓冲区大小（字节）。当 `type == _IONBF` 或 `buf == NULL` 时被忽略
- **C 标准要求**：该函数应当仅在流被打开后、任何其他操作（包括成功的 `setvbuf` 调用）之前调用，否则行为未定义

#### 后置条件

- Case 1 成功（type 为 `_IONBF`）:
  - `f->lbf` 设为 `EOF`（-1）
  - `f->buf_size` 设为 0（无缓冲）
  - `F_SVB` 标志置位
  - 返回 0

- Case 2 成功（type 为 `_IOFBF` 或 `_IOLBF`，且提供了足够大的缓冲区）:
  - `f->lbf` 设为 `EOF`（-1）
  - `f->buf` 设为 `(unsigned char *)(buf + UNGET)`，即实际缓冲区
  - `f->buf_size` 设为 `size - UNGET`
  - 若 `type == _IOLBF` 且 `f->buf_size > 0`，`f->lbf` 进一步设为 `'\n'`
  - `F_SVB` 标志置位
  - 返回 0

- Case 3 成功（type 为 `_IOFBF` 或 `_IOLBF`，但未提供足够大缓冲区或 buf 为 NULL）:
  - `f->lbf` 设为 `EOF`（-1）
  - `f->buf` 和 `f->buf_size` 保持不变（不设置）
  - `F_SVB` 标志置位
  - 返回 0

- Case 4 失败（type 为无效值）:
  - 返回 -1，不修改流状态

#### 系统算法

```
setvbuf(f, buf, type, size):
  1. f->lbf = EOF
  2. 若 type == _IONBF:
       f->buf_size = 0
  3. 否则若 type == _IOLBF 或 type == _IOFBF:
       a. 若 buf 非 NULL 且 size >= UNGET (8):
            f->buf = buf + UNGET
            f->buf_size = size - UNGET
       b. 若 type == _IOLBF 且 f->buf_size > 0:
            f->lbf = '\n'
  4. 否则 (type 无效):
       返回 -1
  5. f->flags |= F_SVB
  6. 返回 0
```

#### 不变量

- `F_SVB` 标志置位后，表明流的缓冲区已由 `setvbuf` 系列函数设置，后续操作不会重新分配内部缓冲区
- `f->buf + UNGET` 之前的 `UNGET` 字节为字符回退预留区，`rpos` 可回退至 `f->buf - UNGET`

#### 依赖

- 无内部函数调用 — 仅直接操作 FILE 结构体字段

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `_IONBF` | 2 | `<stdio.h>` | 无缓冲模式 |
| `_IOLBF` | 1 | `<stdio.h>` | 行缓冲模式 |
| `_IOFBF` | 0 | `<stdio.h>` | 全缓冲模式 |
| `UNGET` | 8 | `stdio_impl.h` | 回退字符预留空间大小 |
| `F_SVB` | 64 | `stdio_impl.h` | setvbuf 已调用标志位 |
| `EOF` | (-1) | `<stdio.h>` | 文件结束标志（也用作 lbf 中表示非行缓冲的哨兵值） |
