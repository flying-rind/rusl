# ungetc.c 规约

> musl libc `ungetc` 实现 — 将一个字符推回 FILE 流的输入缓冲区。这是 ISO C 标准中单字节字符回退的标准接口。

---

## 依赖图

```
ungetc (Public)
  ├── FLOCK / FUNLOCK (锁宏, 定义于 stdio_impl.h)
  │     ├── __lockfile (see __lockfile.c spec)
  │     └── __unlockfile (see __unlockfile.c spec)
  └── __toread (see __toread.c spec)
```

---

## 函数规约

### 1. ungetc

```c
int ungetc(int c, FILE *f);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

将字符 `c` 推回 FILE 流 `f` 的读缓冲区，使得下一次从流中读取时返回该字符。推回的字符被转换为 `unsigned char` 后存放在读缓冲区中。

该函数是标准 C 中唯一的字符回退接口。保证至少可以成功推回一个字符。成功推回后，流的 EOF 状态被清除。

#### 前置条件

- `f`: 有效的 `FILE *` 指针，指向已打开的流
- `c`: 要推回的字符值（以 `int` 传递，仅低 8 位有效）
- 流 `f` 必须处于读模式或有足够空间容纳推回的字符

#### 后置条件

- Case 1 成功推回:
  - `c` 的值（转换为 `unsigned char`）被推回读缓冲区（`f->rpos` 递减并写入）
  - 流的 `F_EOF` 标志被清除
  - 返回转换为 `unsigned char` 的 `c` 值

- Case 2 输入为 EOF:
  - 流状态不变
  - 返回 `EOF`

- Case 3 推回失败（流未处于读模式或缓冲区空间不足）:
  - 流状态不变
  - 返回 `EOF`

#### 系统算法

```
ungetc(c, f):
  1. 若 c == EOF，返回 c（EOF 不可推回）
  2. FLOCK(f) — 获取流锁
  3. 若 f->rpos 为 NULL，调用 __toread(f) 初始化读模式
  4. 若 f->rpos 仍为 NULL（无法进入读模式），或
     f->rpos <= f->buf - UNGET（推回超出预留空间）:
       释放锁，返回 EOF
  5. *--f->rpos = (unsigned char)c — 将字符推回
  6. f->flags &= ~F_EOF — 清除 EOF 标志
  7. FUNLOCK(f) — 释放锁
  8. 返回 (unsigned char)c
```

#### 不变量

- 推回区域始终在缓冲区真实数据之前，`rpos` 可安全回退至 `f->buf - UNGET`
- 成功推回至少一个字符总是可行的，前提是流处于读模式且 `rpos` 有效
- 推回不改变流的错误状态（仅清除 EOF 标志）

#### 依赖

- `__toread(FILE *)` — 将流切换到读模式（定义于 `src/stdio/__toread.c`）
- `__lockfile(FILE *)` / `__unlockfile(FILE *)` — 流加锁/解锁（定义于 `src/stdio/__lockfile.c`）
- `FLOCK` / `FUNLOCK` 宏 — 仅在 `f->lock >= 0` 时加锁/解锁

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `EOF` | (-1) | `<stdio.h>` | 文件结束标志，不可推回 |
| `UNGET` | 8 | `stdio_impl.h` | 字符回退预留空间大小 |
| `F_EOF` | 16 | `stdio_impl.h` | 流 EOF 状态标志位 |
