# fwrite.c 规约

> musl libc 标准 IO 二进制块写入实现。提供内部辅助函数 `__fwritex`（无锁的底层写入引擎）和公开的 `fwrite` / `fwrite_unlocked` 接口。

---

## 依赖图

```
fwrite (Public)
  ├── FLOCK / FUNLOCK (锁宏, 定义于 stdio_impl.h)
  └── __fwritex (Internal)

__fwritex (Internal, hidden)
  ├── __towrite (see __towrite.c spec)
  ├── f->write (FILE 函数指针, typically __stdio_write)
  └── memcpy (from <string.h>)
```

---

## 函数规约

### 1. __fwritex

```c
hidden size_t __fwritex(const unsigned char *restrict s, size_t l, FILE *restrict f);
```

[Visibility]: Internal — `hidden` 属性，仅本模块内可见（被 `fwrite`、`fputs` 等内部调用）

#### Intent

无锁的底层缓冲写入引擎。写入策略如下：
1. **无写缓冲区**: 若 `f->wend` 为 NULL（如未缓冲流），调用 `__towrite` 初始化写模式，失败则返回 0
2. **数据量大于缓冲区剩余空间**: 直接通过 `f->write` 函数指针写入，绕过缓冲
3. **行缓冲模式** (`f->lbf >= 0`): 从末尾向前搜索 `\n`，若有换行则将换行之前的部分通过 `f->write` 刷出，剩余部分拷贝到缓冲区
4. **常规缓冲**: 直接拷贝到 `f->wpos`，推进写指针

#### 前置条件

- `s`: 非空指针，指向至少 `l` 字节的有效数据
- `l`: 要写入的字节数
- `f`: 非空 FILE 指针
- 调用者已持有 `f->lock`（或 FILE 为免锁模式）

#### 后置条件

- **Case 1 完全成功写入**
  - 返回 `l`（全部字节已写入或缓冲）
  - 数据已复制到 FILE 写缓冲区或通过底层 I/O 写入

- **Case 2 行缓冲模式 - 部分刷出**
  - 返回 `l + i`，其中 `i` 是通过 `f->write` 刷出的字节数
  - 剩余数据已缓冲在 `f->wpos`

- **Case 3 写入失败**
  - 返回 0（`__towrite` 失败时）或 `< l`（`f->write` 部分写入）
  - `f->flags` 的 `F_ERR` 可能被设置

#### 系统算法

```
__fwritex(s, l, f):
  i = 0

  // 步骤 1: 确保有写缓冲区
  if f->wend == NULL and __towrite(f) fails: return 0

  // 步骤 2: 数据大于缓冲区剩余空间，直接系统调用写入
  if l > f->wend - f->wpos:
    return f->write(f, s, l)

  // 步骤 3: 行缓冲模式下检查并刷出换行前缀
  if f->lbf >= 0:
    for i = l; i > 0 and s[i-1] != '\n'; i--   // 从末尾找换行
    if i > 0:                                    // 找到换行
      n = f->write(f, s, i)                      // 刷出换行之前的数据
      if n < i: return n                         // 写入失败
      s += i; l -= i                             // 剩余部分将缓冲

  // 步骤 4: 拷贝剩余数据到缓冲区
  memcpy(f->wpos, s, l)
  f->wpos += l
  return l + i
```

**行缓冲逻辑说明**: `f->lbf` 在行缓冲模式下为 `'\n'`（正值），全缓冲时为 `EOF`（负值，通常 -1），无缓冲时缓冲区指针为 NULL。搜索 `s[i-1] != '\n'` 从末尾向前查找非换行，找到最后出现的换行位置 `i`。换行及其之前的数据通过 `f->write` 刷出，换行之后的数据留在缓冲区。

#### 不变量

- `f->wpos` 始终指向写缓冲区的下一个可写位置
- `f->wend` 指向写缓冲区的末尾

#### 依赖

- `__towrite(FILE *)` — 确保 FILE 处于写模式
- `f->write` — FILE 底层写函数指针
- `memcpy` — 内存拷贝（`<string.h>`）

---

### 2. fwrite

```c
size_t fwrite(const void *restrict src, size_t size, size_t nmemb, FILE *restrict f);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

将 `nmemb` 个大小为 `size` 字节的元素从 `src` 写入 FILE 流 `f`。内部加锁后委托给 `__fwritex` 完成实际写入。

#### 前置条件

- `src`: 非空指针（除非 `size` 或 `nmemb` 为 0），指向至少 `size * nmemb` 字节的有效数据
- `size`: 每个元素的大小（字节），可为 0
- `nmemb`: 要写入的元素数量，可为 0
- `f`: 非空 FILE 指针，指向已打开的流（写模式或可写模式）

#### 后置条件

- **Case 1 完全成功写入全部元素**
  - 返回 `nmemb`
  - 所有数据已写入或缓冲

- **Case 2 写入部分元素后出错**
  - 返回 `< nmemb`（实际完整写入的元素数，即 `(已写入字节数) / size`）

- **Case 3 `size` 为 0**
  - 返回 0，不执行任何写操作

#### 系统算法

```
fwrite(src, size, nmemb, f):
  l = size * nmemb                    // 总字节数
  if size == 0: nmemb = 0

  FLOCK(f)                            // 获取 FILE 锁
  k = __fwritex(src, l, f)            // 委托无锁写入引擎
  FUNLOCK(f)                          // 释放锁

  return k == l ? nmemb : k / size    // 计算完整元素数
```

#### 依赖

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁
- `__fwritex(const unsigned char *, size_t, FILE *)` — 同文件 internal 函数

---

### 3. fwrite_unlocked (weak_alias)

```c
// weak_alias(fwrite, fwrite_unlocked);
size_t fwrite_unlocked(const void *restrict src, size_t size, size_t nmemb, FILE *restrict f);
```

[Visibility]: User — POSIX 免锁 `fwrite`，在 musl 中与 `fwrite` 共享同一实现

前置/后置条件及行为：完全等同于 `fwrite`。
