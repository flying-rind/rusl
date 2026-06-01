# regerror.c 规约

## 依赖图

```
regerror (Public API)
  ├── messages[] (static 内部字符串表)
  ├── LCTRANS_CUR() → __lctrans_cur() [外部模块: locale/__lctrans_cur]
  └── snprintf() [外部模块: libc stdio]
```

---

## messages[] 错误消息字符串表

```c
static const char messages[];
```

[Visibility]: Internal — `static` 变量，仅在 regerror.c 内部使用，POSIX/C 标准未定义，不对外导出

### 意图 (Intent)

将全部 14 条错误消息 (对应 `REG_OK` 到 `REG_BADRPT` 的错误码) 以及一条兜底消息合并为单一线性的、以 `\0` 分隔的字符串块，避免运行时的数据重定位 (data relocation)。每条消息以 `\0` 结尾，整个表末尾额外追加一个 `\0`（表现为 `"\0Unknown error"` 中的首字节 `\0`），使得当错误码超出已知范围时能安全落到 "Unknown error"。

### 结构描述

消息按错误码数值顺序排列，错误码的数值即为该消息在表中的索引：

| 索引 | 对应的 `REG_*` 宏 | 消息内容 |
|------|-------------------|----------|
| 0 | `REG_OK` (0) | `"No error"` |
| 1 | `REG_NOMATCH` (1) | `"No match"` |
| 2 | `REG_BADPAT` (2) | `"Invalid regexp"` |
| 3 | `REG_ECOLLATE` (3) | `"Unknown collating element"` |
| 4 | `REG_ECTYPE` (4) | `"Unknown character class name"` |
| 5 | `REG_EESCAPE` (5) | `"Trailing backslash"` |
| 6 | `REG_ESUBREG` (6) | `"Invalid back reference"` |
| 7 | `REG_EBRACK` (7) | `"Missing ']'"` |
| 8 | `REG_EPAREN` (8) | `"Missing ')'"` |
| 9 | `REG_EBRACE` (9) | `"Missing '}'"` |
| 10 | `REG_BADBR` (10) | `"Invalid contents of {}"` |
| 11 | `REG_ERANGE` (11) | `"Invalid character range"` |
| 12 | `REG_ESPACE` (12) | `"Out of memory"` |
| 13 | `REG_BADRPT` (13) | `"Repetition not preceded by valid expression"` |
| 超出范围 (>=14) | (无对应宏) | `"Unknown error"` |

### 不变量 (Invariant)

- 消息索引顺序必须与 `<regex.h>` 中 `REG_*` 错误码的数值定义保持严格一致。
- 第 0 ~ 13 条消息之后必须紧跟一个空字符串 `""` 作为"越界入口"，该空字符串之后才是 `"Unknown error"`，这样当错误码 >= 14 时遍历能自然越过多余的 `\0` 到达兜底消息。

---

## regerror — 将 regex 错误码映射为人类可读的错误消息字符串

```c
size_t regerror(int e, const regex_t *restrict preg, char *restrict buf, size_t size);
```

[Visibility]: Public — POSIX.1-2001 标准函数，声明于 `<regex.h>`（第 56 行），用户程序可直接调用

### 意图 (Intent)

将 `regcomp()` 或 `regexec()` 返回的 `REG_*` 错误码转换为对应的、经过当前 locale 处理的可读错误消息字符串，并将结果写入用户提供的缓冲区。实现上通过遍历以 `\0` 分隔的紧凑字符串表定位对应消息，再经 locale 翻译机制（`__lctrans_cur`）处理后通过 `snprintf` 安全输出，确保不溢出用户缓冲区。

### 前置条件 (Precondition)

- `e` 为 `regcomp()` 或 `regexec()` 的返回值，通常为 `<regex.h>` 中定义的 `REG_*` 宏值（0 ~ 13 或 `REG_ENOSYS` = -1），但允许传入任意 `int` 值。
- `preg` 可为任意值（当前 musl 实现中 `preg` 参数**被完全忽略**，调用者可以传入 `NULL`；但 POSIX 标准要求传递有效的 `regex_t` 指针以保证可移植性）。
- 若 `buf` 非 `NULL` 且 `size > 0`，则 `buf` 指向的缓冲区至少有 `size` 字节可写。
- 若 `size == 0`，`buf` 可以为 `NULL`。

### 后置条件 (Postcondition)

**Case 1: `buf` 非 `NULL` 且 `size > 0`**

- `buf[0 .. min(size-1, ret-1)]` 中写入以 `\0` 结尾的错误消息字符串（经过 locale 处理）。
- 若 `ret <= size`，完整消息被写入 `buf`（含结尾 `\0`）。
- 若 `ret > size`，消息被截断至 `size-1` 字节，`buf[size-1]` 为 `\0`，剩余部分丢失。
- `preg` 参数不影响行为。

**Case 2: `buf == NULL` 或 `size == 0`**

- 不发生写入操作。
- 返回值仍然为完整消息所需的总字符数（含结尾 `\0`），如同 `snprintf(NULL, 0, ...)` 的语义。

**通用:**

- 返回值 = `snprintf(buf, size, "%s", s)` 的返回值，即完整写入消息所需的字符数（含结尾 `\0`）。调用者可以通过比较返回值与 `size` 判断是否发生截断。

### 系统算法 (System Algorithm)

消息定位采用**线性扫描**（而非基于 `REG_*` 数值的直接索引跳转），以紧凑的数据布局换取无动态重定位的代码：

```
设 s 指向 messages 首字节。
对于 i 从 0 到 e-1:
    若 s 指向 '\0':
        停止（已到达兜底入口）
    s 前进 strlen(s) + 1 字节（跳过当前消息及其结尾 '\0'）
若停止时 s 指向 '\0':
    s 前进 1 字节（越过空字符串入口，到达 "Unknown error"）
s = LCTRANS_CUR(s)        // 经过 locale 翻译处理
返回 1 + snprintf(buf, size, "%s", s)
```

**边界情况说明**：

- 当 `e == 0`：for 循环不执行，`s` 直接指向 `"No error"`（索引 0），输出 `"No error"`。
- 当 `e == -1`（即 `REG_ENOSYS`）：实际遍历时 `e` 为 -1，但 `e` 在 for 循环条件 `e && *s` 中，第一次迭代后 `e` 递减为 -2，`*s` 为 `'N'`（非零），继续循环。循环会遍历全部 14 条已知消息直到遇空字符串（第14个入口），经 locale 处理后输出。但实际上 -1 对于 `size_t` 类型比较时会发生隐式转换——等等，仔细分析：`e` 是 `int`，`--e` 是 int 运算，不存在 size_t 问题。for 循环条件 `e && *s` 中，当 `e = -1` 时 `e` 为真（非零），进入循环体一次，`e--` 后 `e = -2`。下次检查 `e`，-2 仍为真，继续循环...实际上，`e` 作为 for 循环计数器递减时会在遇到 `*s == '\0'` 时停止。如果 `e` 为负数（如 REG_ENOSYS = -1），循环会一直执行直到读取到 messages 中首个 `\0`（即第14条消息之后的空入口），然后 `s` 前进到 `"Unknown error"`。**注意**：这是一个未定义行为边界——如果调用者传入极大正数 `e`（比如 > 14），循环同样会到达空入口并返回 `"Unknown error"`，所以实际上任何非 [0, 13] 的 `e` 值都会返回 `"Unknown error"`。

**关键设计决策**：

1. **紧凑字符串表**：用单一 `static const char[]` 存放所有消息，编译器将其放入 `.rodata` 段，避免了独立字符串指针数组在加载时需要的动态重定位。
2. **locale 支持**：通过 `LCTRANS_CUR`（展开为 `__lctrans_cur`）将消息送入 musl 的 locale 翻译机制，使消息文本能根据当前 locale 设置进行翻译。
3. **`preg` 参数忽略**：musl 当前实现完全忽略 `preg` 参数。这在 POSIX 标准中是允许的（标准表明此参数可能用于获取 locale 信息，但非强制）。这一简化是有意为之，因为 musl 的所有 locale 信息通过全局线程局部存储获取，无需从 `preg` 中提取。

### 使用示例

```c
#include <regex.h>
#include <stdio.h>

int main() {
    regex_t regex;
    int err = regcomp(&regex, "[invalid", REG_EXTENDED);
    if (err != 0) {
        char errbuf[256];
        regerror(err, &regex, errbuf, sizeof(errbuf));
        fprintf(stderr, "regcomp failed: %s\n", errbuf);
        // 输出: regcomp failed: Missing ']'
    }
    // ...

    // 也可以用于获取所需缓冲区大小而不实际写入:
    size_t needed = regerror(REG_ESPACE, NULL, NULL, 0);
    // needed == 15 (13 字符 "Out of memory" + '\0' + snprintf 返回含 '\0' 的长度)
}
```
