# vwprintf.c 规约

> musl libc 宽字符标准输出格式化函数（va_list 版本）。直接委托给 `vfwprintf(stdout, ...)`。

---

## 依赖图

```
vwprintf (Public)
  └─> vfwprintf(stdout, fmt, ap)  (see vfwprintf.c spec)

stdout (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### 1. vwprintf

```c
int vwprintf(const wchar_t *restrict fmt, va_list ap);
```

[Visibility]: User — `<stdarg.h>` / `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

将格式化宽字符串输出到标准输出流 `stdout`。是 `wprintf` 的 `va_list` 版本。直接委托给 `vfwprintf(stdout, fmt, ap)`。

#### 前置条件

- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化
- `stdout` 已初始化，可写入

#### 后置条件

- Case 1 成功：返回写入 `stdout` 的宽字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`

#### 系统算法

```
vwprintf(fmt, ap):
  return vfwprintf(stdout, fmt, ap)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vfwprintf()` — 宽字符格式化输出核心引擎（见 `vfwprintf.c`）
- `stdout` — 标准输出流（见 `src/stdio/__stdout_used.c`）
