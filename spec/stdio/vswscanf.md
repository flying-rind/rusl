# vswscanf.c 规约

> musl libc 宽字符串格式化输入函数（va_list 版本）。创建自定义只读 FILE 流，从用户提供的宽字符串缓冲区读取格式化输入。

---

## 依赖图

```
vswscanf (Public)
  ├─> wstring_read (static) — 读取回调
  │     ├─> wcsrtombs (来自 <wchar.h>) — 宽字符串到多字节转换
  │     └─> f->buf / f->rpos / f->rend / f->cookie — FILE 缓冲区管理
  └─> vfwscanf (see vfwscanf.c) — 格式化引擎

__isoc99_vswscanf (weak_alias)
  └─> vswscanf
```

---

## 函数规约

### 1. wstring_read (static)

```c
static size_t wstring_read(FILE *f, unsigned char *buf, size_t len);
```

[Visibility]: Internal (不导出) — 自定义 FILE 读取回调

#### Intent

按需将宽字符串源转换为多字节数据，供 `vfwscanf` 内部 `fscanf` 使用。采用惰性策略：每次只将前一阶段转换后的一个字节返回给调用者，下次请求时再转换下一批。

#### 前置条件

- `f` 指向有效的自定义 FILE 对象（由 `vswscanf` 在栈上创建）
- `f->cookie` 指向宽字符串源的当前位置

#### 后置条件

- **Case 1 有数据可读** — 返回 `1`，`*buf` 包含一个字节
  - `f->rpos` 已前进
  - `f->cookie` 已更新为下一批宽字符的起始位置
- **Case 2 到达字符串末尾** — 返回 `0`
  - `f->rpos = f->rend = 0`
- **Case 3 多字节转换错误** — 返回 `0`
  - `f->rpos = f->rend = 0`

#### 系统算法

```
wstring_read(f, buf, len):
  src = f->cookie
  if (!src): return 0

  // 转换一批宽字符为多字节（使用 FILE 缓冲区）
  k = wcsrtombs(f->buf, &src, f->buf_size, 0)
  if (k == (size_t)-1):
    f->rpos = f->rend = 0
    return 0

  f->rpos = f->buf           // 设置读指针
  f->rend = f->buf + k
  f->cookie = (void *)src    // 更新 cookie 为下一个位置

  if (!len || !k): return 0

  *buf = *f->rpos++          // 返回一个字节
  return 1
```

#### 依赖

- `wcsrtombs()` — 宽字符串到多字节字符串转换，支持状态（`<wchar.h>`）

---

### 2. vswscanf

```c
int vswscanf(const wchar_t *restrict s, const wchar_t *restrict fmt, va_list ap);
```

[Visibility]: User — `<stdarg.h>` / `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从宽字符串 `s` 读取格式化输入。通过创建自定义 FILE 对象并设置 `wstring_read` 为读取回调，将 `vfwscanf` 的输入源重定向到用户提供的宽字符串。

#### 前置条件

- `s != NULL`，指向以 `L'\0'` 结尾的有效宽字符串
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串末尾）：返回 `EOF`
- `s` 不被修改（只读）

#### 系统算法

```
vswscanf(s, fmt, ap):
  buf[256]                 // 栈上 FILE 缓冲区
  FILE f = {
    .buf = buf,
    .buf_size = sizeof buf,
    .cookie = (void *)s,   // 指向宽字符串源
    .read = wstring_read,  // 自定义读取回调
    .lock = -1,            // 免锁 FILE
  }
  return vfwscanf(&f, fmt, ap)
```

#### 不变量

- 自定义 FILE 为免锁模式（`lock = -1`）
- 自定义 FILE 使用 `buf_size` 字节的栈上缓冲区作为转换中间件
- `wcsrtombs` 在缓冲区填满或宽字符串耗尽时停止，下一次 `wstring_read` 调用从上次停止位置继续

#### 依赖

- `vfwscanf()` — 宽字符格式化输入核心引擎（见 `vfwscanf.c`）
- `wstring_read()` (static) — 自定义读取回调（同文件）
- `wcsrtombs()` — 宽字符串到多字节字符串转换（`<wchar.h>`）

---

### 3. __isoc99_vswscanf (weak_alias)

```c
weak_alias(vswscanf, __isoc99_vswscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部 C99 兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_vswscanf` 弱别名。与 `vswscanf` 行为完全相同。
