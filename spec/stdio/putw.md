# putw.c 规约

> musl libc 整数二进制写入实现。将一个 int 整数的二进制表示写入 FILE 流。

---

## 依赖图

```
putw (Public — _GNU_SOURCE)
  └── fwrite(&x, sizeof x, 1, f)  — 块写入 (src/stdio/fwrite.c)
```

---

## 函数规约

### 1. putw

```c
#define _GNU_SOURCE
int putw(int x, FILE *f);
```

[Visibility]: User — SVID 兼容 / GNU 扩展函数，需定义 `_GNU_SOURCE` 方可使用。非 ISO C/POSIX 标准

#### Intent

将 `int` 值 `x` 的底层二进制表示直接写入 FILE 流 `f`。使用 `fwrite` 写入 `sizeof(int)` 字节，返回值编码为：成功返回 `0`，失败返回 `EOF`。

**注意**: 这不是格式化 I/O。写入的是整数的二进制字节（依赖平台字节序和 sizeof(int)），而非其十进制文本表示。数据不具有可移植性。

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的写模式流
- `x`: 任意 int 值（写入其二进制表示）

#### 后置条件

- **Case 1 写入成功（全部 `sizeof(int)` 字节写入）**
  - `x` 的二进制表示已写入流 `f`
  - 返回 `0`

- **Case 2 写入失败（零或部分字节写入）**
  - 返回 `EOF`（通常为 `-1`）
  - `f` 的错误标志可能被设置

#### 系统算法

```
putw(x, f):
  // fwrite 返回成功写入的对象数 (0 或 1)
  // 成功时返回 (1) - 1 = 0
  // 失败时返回 (0) - 1 = -1 (EOF)
  return (int)fwrite(&x, sizeof x, 1, f) - 1
```

#### 不变量

- 始终写入 `sizeof(int)` 字节（平台相关，通常为 4 字节）
- 数据格式为原生字节序的二进制整数
- `putw` 本身不锁定 FILE（锁由 `fwrite` 管理）

#### 依赖

- `fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream)` — 块写入（定义于 `src/stdio/fwrite.c`）
- `sizeof(int)` — 平台 int 大小（编译器内置）

#### 返回值对照

| 返回值 | 含义 |
|--------|------|
| `0` | 写入成功（1 个对象写入，`1 - 1 = 0`） |
| `-1` (EOF) | 写入失败（0 个对象写入，`0 - 1 = -1`） |
