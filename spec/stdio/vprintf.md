# vprintf.c 规约

> musl libc `va_list` 版标准输出格式化函数。直接委托 `vfprintf(stdout, ...)`。

---

## 依赖图

```
vprintf
  └─> vfprintf(stdout, fmt, ap)  (see vfprintf.c spec)
```

---

## 函数规约

### 1. vprintf

```c
int vprintf(const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

将格式化字符串输出到标准输出流 `stdout`（`va_list` 版本）。是 `printf` 的 `va_list` 平替。

#### 前置条件

- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化
- `stdout` 已初始化，可写入

#### 后置条件

- Case 1 成功：返回写入 `stdout` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`

#### 系统算法

```
vprintf(fmt, ap):
  return vfprintf(stdout, fmt, ap)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vfprintf()` — 格式化输出核心引擎（见 `vfprintf.c`）
- `stdout` — 标准输出流（见 `src/stdio/__stdout_used.c`）
