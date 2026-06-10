# gets.c 规约

> musl libc 标准库字符串输入（无边界检查）实现。从 stdin 读取一行到用户缓冲区。

---

## 依赖图

```
gets (Public)
  ├── getc_unlocked(stdin)              — 从 stdin 无锁读取一个字符 (stdio_impl.h 宏 / src/stdio/fgetc.c)
  ├── feof(stdin)                       — 检查 stdin 是否到达文件末尾 (src/stdio/feof.c)
  └── FLOCK(stdin) / FUNLOCK(stdin)     — stdin 锁定/解锁 (stdio_impl.h)
```

---

## 函数规约

### 1. gets

```c
char *gets(char *s);
```

[Visibility]: User — `<stdio.h>` 标准库函数（C99 后标记为过时，C11 已移除），用户程序可直接调用

#### Intent

从 `stdin` 读取字符直到遇到换行符 `\n` 或文件末尾，将读取的字符（不包括 `\n`）存入用户缓冲区 `s`，并在末尾添加 `\0` 终止符。若既未读到换行符也未读到任何字符就遇到 EOF，返回 `NULL`。

**严重警告**: `gets` 不对缓冲区进行边界检查，无法安全使用。任何输入超过缓冲区大小的场景都会导致缓冲区溢出。C11 标准已移除该函数，POSIX.1-2008 标记为过时。必须使用 `fgets` 替代。

#### 前置条件

- `s`: 非空指针，指向足够大的字符数组（无大小限制信息可用，这是不安全的根本原因）
- `stdin` 已初始化且可读

#### 后置条件

- **Case 1 成功读取（读到换行符或 EOF 前有数据）**
  - `s` 中包含读取的字符（去除了末尾 `\n`），以 `\0` 结尾
  - 返回 `s`（指向用户缓冲区的指针）
  - `\n` 字符被从流中消耗但**不**存入缓冲区

- **Case 2 读取失败（遇到 EOF 且未读到任何字符，或读 I/O 错误）**
  - 返回 `NULL`
  - `s` 的内容未定义或为 `\0`（musl 实现中，若读到 0 个字符 + EOF，`s[0] = '\0'`）
  - `stdin` 的 EOF 或 error 标志被设置

#### 系统算法

```
gets(s):
  1. i = 0
  2. FLOCK(stdin)                      // 锁定 stdin

  3. while (c = getc_unlocked(stdin)) != EOF && c != '\n':
       s[i++] = c

  4. s[i] = '\0'                       // 终止符

  5. if (c != '\n' && (!feof(stdin) || !i)):
       // 情况A: c == EOF 且不是因为换行符结束
       //        并且 (stdin 并非真正 EOF 或者 i==0 即没读到任何字符)
       //        即：I/O 错误(-1) 或 零字节 EOF
       s = NULL                         // 返回 NULL
     // 情况B: 正常读到换行符 (\n 被丢弃, 返回 s)
     // 情况C: 读到数据后遇到真正 EOF (i>0, feof 为真): 返回 s

  6. FUNLOCK(stdin)
  7. return s
```

**失败判断逻辑分析**:

| c 最终值 | feof(stdin) | i | 返回值 | 说明 |
|----------|------------|---|--------|------|
| `'\n'` | — | 任意 | `s` | 正常遇到换行符，成功 |
| `EOF` | true | `>0` | `s` | 读到数据后遇到 EOF |
| `EOF` | true | `0` | `NULL` | 遇到 EOF，未读到任何数据 |
| `EOF` | false | 任意 | `NULL` | I/O 读取错误 |

条件 `(c != '\n' && (!feof(stdin) || !i))` 的语义：
- `c != '\n'` 意味着以 EOF 结束
- `!feof` 表示不是真正的文件末尾（即读错误）
- `!i` 表示没有读到任何字符
- 所以当读错误或空 EOF 时返回 NULL

#### 不变量

- 始终从 `stdin` 读取
- 换行符 `\n` 被消耗但不写入缓冲区
- 缓冲区始终以 `\0` 终止（在返回之前）
- **无缓冲区边界检查** — 这是严重安全漏洞的根源

#### 依赖

- `getc_unlocked(FILE *stream)` — 无锁读取单个字符（宏/函数，定义于 `src/stdio/fgetc.c` 或 `getc.h`）
- `feof(FILE *stream)` — 检查文件结束标志（宏/函数，定义于 `src/stdio/feof.c`）
- `stdin` — 标准输入流 FILE 指针（全局变量，定义于 `src/stdio/__stdin_used.c`）
- `FLOCK(FILE *f)` / `FUNLOCK(FILE *f)` — 获取/释放 FILE 锁（宏，定义于 `src/internal/stdio_impl.h`）
- `EOF` — 文件结束/错误返回值常量（来自 `<stdio.h>`）

#### 安全说明

`gets` 是 C 语言历史上最危险的标准库函数之一。由于没有缓冲区大小参数，无法预防缓冲区溢出。1988 年 Morris 蠕虫即利用 `gets` 漏洞传播。该函数已在 ISO C11 中完全移除。
