# \_\_overflow.c 规约

> musl libc 内部输出缓冲区溢出处理实现。当 `putc_unlocked` 宏检测到写缓冲区满或需要特殊处理时调用，负责将单个字符写入 FILE 流。

---

## 依赖图

```
__overflow
  ├─> __towrite     (see __towrite.c spec)
  └─> f->write      (see __stdio_write.c / __stdout_write.c spec)
```

---

## 函数规约

### 1. \_\_overflow

```c
int __overflow(FILE *f, int _c);
```

[Visibility]: Internal — musl 内部实现，但编译为 `protected` 可见性（非 hidden），因为被 `putc_unlocked` 宏直接引用。用户代码通过宏间接调用。

#### Intent

处理 stdio 输出缓冲区的"溢出"情况。分为三种场景：
1. 流尚未初始化写模式（`f->wend == 0`）：切换到写模式
2. 写缓冲区还有空间，且字符不是行缓冲字符（`c != f->lbf`）：直接写入缓冲区
3. 写缓冲区满，或字符触发行缓冲刷新：调用 `f->write` 写出

#### 前置条件

- `f`: `FILE*`，非 NULL
- `_c`: 要写入的字符（作为 `int` 传递，内部转为 `unsigned char`）
- `f->write` 已设置为有效的写函数（如 `__stdio_write` 或 `__stdout_write`）

#### 后置条件

**Case 1: 成功写入**

- 字符被写入缓冲区或通过 `f->write` 写出
- 返回写入的字符值（`c`，即原字符转为 `unsigned char` 后的值）

**Case 2: 失败**

- 若 `__towrite` 失败（写模式切换失败）：返回 `EOF`
- 若 `f->write` 写出的字节数不为 1：返回 `EOF`

#### 系统算法

```
__overflow(f, _c):
  c = (unsigned char)_c

  /* 1. 流尚未初始化写模式 */
  if !f->wend && __towrite(f):
    return EOF

  /* 2. 缓冲区有空间，且字符不触发行缓冲 */
  if f->wpos != f->wend && c != f->lbf:
    return *f->wpos++ = c        // 直接存入缓冲区

  /* 3. 缓冲区满或行缓冲触发：调用函数指针写出 */
  if f->write(f, &c, 1) != 1:
    return EOF
  return c
```

#### 行缓冲触发语义

当 `f->lbf == '\n'` 且 `c == '\n'` 时，即使缓冲区有空间，触发条件 `c != f->lbf` 为假，导致走 Case 3（`f->write` 路径），从而将缓冲区内容连同换行符一并刷新。这实现了 POSIX 行缓冲语义。

#### 不变量

- 调用 `f->write` 时始终传入单字节数据（`&c, 1`）
- `f->write` 返回 1 时，内部缓冲区已由 `__stdio_write` 处理（重置 `wpos`/`wbase`）

#### 依赖

- `__towrite()` — 将流切换为写模式（本模块，see `__towrite.c` spec）
- `f->write` — 流写函数指针（通常为 `__stdio_write` 或 `__stdout_write`）
- `EOF` — 标准文件结束/错误常量（`<stdio.h>`）
