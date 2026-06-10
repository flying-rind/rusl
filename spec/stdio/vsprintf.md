# vsprintf.c 规约

> musl libc 字符串格式化输出函数（`va_list` 版本，无边界检查）。通过将 `INT_MAX` 传给 `vsnprintf` 实现。

---

## 依赖图

```
vsprintf
  └─> vsnprintf(s, INT_MAX, fmt, ap)  (see vsnprintf.c spec)
```

---

## 函数规约

### 1. vsprintf

```c
int vsprintf(char *restrict s, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

将格式化字符串写入用户提供的缓冲区 `s`（`va_list` 版本）。不执行边界检查。

#### 前置条件

- `s` 指向足够大的可写缓冲区（调用者保证）
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：返回写入 `s` 的字符总数（不含 `'\0'`），`s` 以 `'\0'` 结尾
- Case 2 失败：返回负值
- 行为等价于 `vsnprintf(s, INT_MAX, fmt, ap)`

#### 系统算法

```
vsprintf(s, fmt, ap):
  return vsnprintf(s, INT_MAX, fmt, ap)
```

#### 不变量

无。本函数纯粹作为转发代理（`INT_MAX` 作为 size 参数传入 `vsnprintf`）。

#### 依赖

- `vsnprintf()` — 有边界检查的格式化输出（见 `vsnprintf.c`）
- `INT_MAX` — 定义于 `<limits.h>`
