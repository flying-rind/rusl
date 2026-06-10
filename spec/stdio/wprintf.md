# wprintf.c 规约

> musl libc 宽字符标准输出格式化函数。是 `vwprintf(fmt, ...)` 的可变参数包装，最终委托给 `vfwprintf(stdout, ...)`。

---

## 依赖图

```
wprintf (Public)
  └─> vwprintf(fmt, ap)  (see vwprintf.c spec)
        └─> vfwprintf(stdout, fmt, ap)  (see vfwprintf.c spec)
```

---

## 函数规约

### 1. wprintf

```c
int wprintf(const wchar_t *restrict fmt, ...);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

将格式化宽字符串输出到标准输出流 `stdout`。是 `vwprintf` 的可变参数包装器。与 `printf` 的区别在于格式字符串为宽字符。

#### 前置条件

- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `stdout` 已初始化，可写入
- 可变参数与格式串匹配

#### 后置条件

- Case 1 成功：返回写入 `stdout` 的宽字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
wprintf(fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vwprintf(fmt, ap) 委托 vwprintf
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vwprintf()` — 宽字符标准输出格式化函数（见 `vwprintf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏
