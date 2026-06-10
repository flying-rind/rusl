# sprintf.c 规约

> musl libc 字符串格式化输出函数（无边界检查）。是 `vsprintf(s, ...)` 的可变参数包装。

---

## 依赖图

```
sprintf
  └─> vsprintf(s, fmt, ap)  (see vsprintf.c spec)
        └─> vsnprintf(s, INT_MAX, fmt, ap)  (see vsnprintf.c spec)
```

---

## 函数规约

### 1. sprintf

```c
int sprintf(char *restrict s, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

将格式化字符串写入用户提供的缓冲区 `s`。不执行边界检查，用户必须确保缓冲区足够大以容纳完整输出。

#### 前置条件

- `s` 指向足够大的可写缓冲区（由调用者保证，无自动截断）
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配

#### 后置条件

- Case 1 成功：返回写入 `s` 的字符总数（不含 `'\0'`），`s` 以 `'\0'` 结尾
- Case 2 失败：返回负值
- `va_list` 在返回前已通过 `va_end` 清理

#### 不变量

无。本函数纯粹作为转发代理。

#### 系统算法

```
sprintf(s, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vsprintf(s, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

#### 依赖

- `vsprintf()` — 无边界检查的格式化输出（见 `vsprintf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏
