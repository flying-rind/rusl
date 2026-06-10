# puts.c 规约

> musl libc 标准库字符串输出（带换行）实现。向 stdout 输出字符串并自动追加换行符。

---

## 依赖图

```
puts (Public)
  ├── fputs(s, stdout)              — 写入字符串到 stdout (src/stdio/fputs.c)
  ├── putc_unlocked('\n', stdout)   — 写入换行符 (stdio_impl.h 宏 / src/stdio/fputc.c)
  └── FLOCK(stdout) / FUNLOCK(stdout) — stdout 锁定/解锁 (stdio_impl.h)
```

---

## 函数规约

### 1. puts

```c
int puts(const char *s);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

将 C 字符串 `s`（不含结尾 `\0`）写入 `stdout`，随后自动写入一个换行符 `\n`。与 `fputs` 的区别在于：自动追加换行，始终写入 `stdout`（而非任意流）。

#### 前置条件

- `s`: 非空指针，指向以 `\0` 结尾的有效 C 字符串
- `stdout` 已初始化且可写
- 调用线程可以获取 `stdout` 的锁

#### 后置条件

- **Case 1 成功**
  - `s` 的内容后跟一个 `\n` 已写入 `stdout`
  - 返回非负值（musl 实现中成功时返回 `0`，因为 `!(0) = 1`，`-(1) = -1` 的逻辑相反；实际 `fputs` 成功返回 `0`，`putc_unlocked` 成功返回 `'\n'`（非负））

- **Case 2 输出失败**
  - 返回 `EOF`（负值，通常为 `-1`）
  - `stdout` 出错标志可能被设置

#### 系统算法

```
puts(s):
  1. FLOCK(stdout)                     // 锁定 stdout

  2. r = -(fputs(s, stdout) < 0 || putc_unlocked('\n', stdout) < 0)
     // fputs 失败返回 EOF(<0)
     // putc_unlocked 失败返回 EOF(<0)
     // 任一失败则括号内表达式为 true(1)，-(1) = -1 (EOF)
     // 两者都成功则括号内为 false(0)，-(0) = 0（成功）

  3. FUNLOCK(stdout)                   // 解锁 stdout

  4. return r
```

**返回值技巧解析**:
- `fputs` 成功返回 `0`（非负），`< 0` 为 false
- `putc_unlocked` 成功返回写入的字符（如 `'\n'`=10，非负），`< 0` 为 false
- 两个 false 的 OR 结果为 false(0)，取负得 `-0 = 0`
- 任一调用失败返回 `EOF`(-1)，对应 `< 0` 为 true(1)，OR 结果为 true(1)，取负得 `-1`

#### 不变量

- `puts` 始终向 `stdout` 输出，不向其他流
- 输出始终以 `\n` 结束（无论输入字符串是否以 `\n` 结尾）
- `puts` 本身持有 `stdout` 锁

#### 依赖

- `fputs(const char *restrict s, FILE *restrict stream)` — 写入字符串到 FILE（定义于 `src/stdio/fputs.c`）
- `putc_unlocked(int c, FILE *stream)` — 无锁写入单个字符（宏/函数，定义于 `src/stdio/fputc.c` 或 `putc.h`）
- `stdout` — 标准输出流 FILE 指针（全局变量，定义于 `src/stdio/__stdout_used.c`）
- `FLOCK(FILE *f)` / `FUNLOCK(FILE *f)` — 获取/释放 FILE 锁（宏，定义于 `src/internal/stdio_impl.h`）
- `EOF` — 文件结束/错误返回值常量（来自 `<stdio.h>`）
