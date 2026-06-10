# getw.c 规约

> musl libc 整数二进制读取实现。从 FILE 流中读取一个 int 整数的二进制表示。

---

## 依赖图

```
getw (Public — _GNU_SOURCE)
  └── fread(&x, sizeof x, 1, f)  — 块读取 (src/stdio/fread.c)
```

---

## 函数规约

### 1. getw

```c
#define _GNU_SOURCE
int getw(FILE *f);
```

[Visibility]: User — SVID 兼容 / GNU 扩展函数，需定义 `_GNU_SOURCE` 方可使用。非 ISO C/POSIX 标准

#### Intent

从 FILE 流 `f` 中读取 `sizeof(int)` 字节的二进制数据，将其解释为一个 `int` 值并返回。若读取失败（EOF 或错误），返回 `EOF`。

**注意**: 这是对整数的二进制读取，读取的是平台原生字节序的 `sizeof(int)` 字节。不可便携——在不同平台/字节序的系统之间交换数据无效。调用者需自行区分 `EOF` 返回值是真正的文件结束还是碰巧读到的 `int` 值为 `EOF`。

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的读模式流

#### 后置条件

- **Case 1 读取成功（完整读取 `sizeof(int)` 字节）**
  - `fread` 返回 1（成功读取 1 个对象）
  - 返回读取的 `int` 值（即 `x`）

- **Case 2 读取失败（0 字节或部分字节，或立即遇到 EOF）**
  - `fread` 返回 0
  - 返回 `EOF`（通常为 `-1`）
  - `f` 的 EOF 或错误标志可能被设置

#### 系统算法

```
getw(f):
  int x
  if fread(&x, sizeof x, 1, f) == 1:  // 成功读取 1 个 int
    return x                           // 返回读取的值
  else:
    return EOF                         // 失败
```

等价于 `return fread(&x, sizeof x, 1, f) ? x : EOF;`——利用 C 语言布尔语义：`fread` 成功返回 1（真），失败返回 0（假）。

#### 不变量

- 始终尝试读取恰好 `sizeof(int)` 字节
- 读取的字节按平台原生字节序解释为 `int`
- `getw` 本身不锁定 FILE（锁由 `fread` 管理）

#### 依赖

- `fread(void *ptr, size_t size, size_t nmemb, FILE *stream)` — 块读取（定义于 `src/stdio/fread.c`）
- `sizeof(int)` — 平台 int 大小（编译器内置）
- `EOF` — 文件结束常量（来自 `<stdio.h>`）

#### 注意事项

- `EOF` 通常定义为 `-1`，而 `-1` 本身也是一个合法的 `int` 值。因此若流中实际读出 `-1`，调用者无法区分是读取到了值 `-1` 还是遇到了文件结束。应使用 `feof(f)` 或 `ferror(f)` 进行区分。
