# getline.c 规约

> musl libc 标准换行符分隔的动态行读取实现。`getline` 是 `getdelim` 的特化版本，固定以 `\n` 为分隔符。

---

## 依赖图

```
getline (Public)
  └── getdelim (see getdelim.c spec)
        ├── FLOCK / FUNLOCK
        ├── memchr / memcpy
        ├── realloc
        ├── getc_unlocked
        ├── feof
        └── errno
```

---

## 函数规约

### getline

```c
ssize_t getline(char **restrict s, size_t *restrict n, FILE *restrict f);
```

[Visibility]: User — POSIX.1-2008 标准函数，用户程序包含 `<stdio.h>` 即可调用

#### Intent

从 FILE 流 `f` 中读取以 `\n`（换行符）结尾的一行数据。等价于 `getdelim(s, n, '\n', f)`。详细信息参见 [getdelim.c.md](./getdelim.md)。

#### 前置条件

- `s`: 非空指针，指向 `char *` 变量（该变量可为 NULL 或指向已有缓冲区）
- `n`: 非空指针，指向 `size_t` 变量（指示当前缓冲区大小）
- `f`: 非空 FILE 指针，指向已打开的读模式流

#### 后置条件

- **Case 1 成功读取一行（以换行符结尾）**
  - 返回读取的字符数（**包括** `\n`，不包括结尾的 `\0`）
  - `*s` 指向包含完整行 + `\0` 的动态分配缓冲区
  - `*n` 更新为当前缓冲区大小

- **Case 2 成功读取到文件末尾（无换行符，如文件末尾缺少换行）**
  - 返回读取的字符数（不包括 `\0`）
  - `*s` 指向包含数据 + `\0` 的缓冲区
  - FILE 流设置 `F_EOF` 标志

- **Case 3 参数无效 (`s` 或 `n` 为 NULL)**
  - 返回 -1，`errno = EINVAL`

- **Case 4 内存不足或读取错误**
  - 返回 -1

#### 系统算法

```
getline(s, n, f):
  return getdelim(s, n, '\n', f)
```

#### 依赖

- `getdelim(char **, size_t *, int, FILE *)` — 通用分隔符行读取函数（定义于 `getdelim.c`）
