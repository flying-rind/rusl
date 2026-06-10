# vsscanf.c 规约

> musl libc `va_list` 版字符串格式化输入函数。通过构造最小伪 FILE 对象并委托 `vfscanf` 实现。

---

## 依赖图

```
vsscanf
  ├─> string_read (static) — 从字符串中读取的 FILE 回调
  │     └─> memchr (see <string.h>) — 查找 '\0' 终止符
  ├─> 构造 FILE (栈上)
  └─> vfscanf(&f, fmt, ap)  (see vfscanf.c spec)

__isoc99_vsscanf (weak_alias)
  └─> vsscanf
```

---

## 内部函数规约

### 1. string_read (static)

```c
static size_t string_read(FILE *f, unsigned char *buf, size_t len);
```

[Visibility]: Internal — 不对外导出

#### Intent

作为 `FILE` 的读取回调，从内存中的 null 结尾字符串提供数据。模拟从文件读取的行为。

#### 前置条件

- `f->cookie` 指向有效的 null 结尾 C 字符串
- `f->rpos` / `f->rend` 跟踪当前读取位置

#### 后置条件

- 最多 `len` 字节从源字符串拷贝到 `buf`
- 若源字符串在 `len` 字节内结束，返回较少字节数
- 更新 `f->rpos`、`f->rend`、`f->cookie` 以反映新的读取位置

#### 系统算法

```
string_read(f, buf, len):
  src = f->cookie
  // 在 len+256 范围内查找 '\0' 以确定可用数据长度
  k = len + 256
  end = memchr(src, 0, k)
  if end: k = end - src
  if k < len: len = k
  memcpy(buf, src, len)
  f->rpos = src + len
  f->rend = src + k
  f->cookie = src + k
  return len
```

---

## 函数规约

### 2. vsscanf

```c
int vsscanf(const char *restrict s, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

从内存中的 null 结尾字符串 `s` 读取格式化输入（`va_list` 版本）。是 `sscanf` 的 `va_list` 平替。

#### 前置条件

- `s` 指向有效的 null 结尾 C 字符串
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串结尾）：返回 `EOF`
- `s` 源字符串不会被修改

#### 系统算法

```
vsscanf(s, fmt, ap):
  1. 在栈上构造 FILE 对象：
     .buf = (void *)s      // 缓冲区直接指向源字符串
     .cookie = (void *)s   // cookie 追踪读取位置
     .read = string_read   // 自定义读取回调
     .lock = -1            // 禁用锁定（伪流不会被共享）
  2. return vfscanf(&f, fmt, ap)
```

#### 不变量

- `FILE` 对象仅在栈上存在，函数返回后销毁
- 源字符串 `s` 为只读，不被修改
- 无锁模式（`lock = -1`），伪流不会被多个线程共享

#### 依赖

- `vfscanf()` — 格式化输入核心引擎（见 `vfscanf.c`）
- `string_read()` (static) — 自定义字符串读取回调
- `memchr()` — 查找字符串终止符（见 `src/string/memchr.c`）
- `stdio_impl.h` — `FILE` 结构体定义

---

### 3. __isoc99_vsscanf (weak_alias)

```c
weak_alias(vsscanf, __isoc99_vsscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_vsscanf` 弱别名。与 `vsscanf` 行为完全相同。
