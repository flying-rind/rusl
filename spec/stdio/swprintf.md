# swprintf.c 规约

> musl libc 宽字符字符串格式化输出函数。是 `vswprintf(s, n, fmt, ...)` 的可变参数包装。

---

## 依赖图

```
swprintf (Public)
  └─> vswprintf(s, n, fmt, ap)  (see vswprintf.c spec)
```

---

## 函数规约

### 1. swprintf

```c
int swprintf(wchar_t *restrict s, size_t n, const wchar_t *restrict fmt, ...);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

将格式化宽字符串输出到缓冲区 `s`，最多写入 `n` 个宽字符（含终止 `L'\0'`）。是 `vswprintf` 的可变参数包装器。与 `snprintf` 类似但有两点关键区别：
1. 返回值：成功时返回写入的宽字符数（不含 `L'\0'`），若 `ret >= n` 则表示输出被截断，但 swprintf 在截断时返回 `-1` 而非截断前的完整长度（不同于 C99 标准要求）
2. 格式字符串和目标缓冲区均为宽字符

#### 前置条件

- `s`: 指向有效宽字符缓冲区的指针（`n > 0` 时）；`n == 0` 时可为 `NULL`
- `n`: 缓冲区大小（宽字符数）
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配

#### 后置条件

- Case 1 成功（输出未截断）：返回写入 `s` 的宽字符数（不含 `L'\0'`）
- Case 2 截断（`ret >= n`）：返回 `-1`（musl 行为，非 C99 标准）
- Case 3 `n == 0`：返回 `-1`
- Case 4 输出错误：返回 `-1`
- Case 5 格式错误：返回 `-1`，`errno = EINVAL`
- Case 6 溢出：返回 `-1`，`errno = EOVERFLOW`
- `s` 被 `L'\0'` 终止（当 `n > 0` 时）
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
swprintf(s, n, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vswprintf(s, n, fmt, ap) 委托核心引擎
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vswprintf()` — 宽字符字符串格式化输出核心引擎（见 `vswprintf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏
