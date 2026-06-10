# fprintf.c 规约

> musl libc 文件流格式化输出函数。是 `vfprintf(f, ...)` 的可变参数包装。

---

## 依赖图

```
fprintf
  └─> vfprintf(f, ...)  (see vfprintf.c spec)
```

---

## 函数规约

### 1. fprintf

```c
int fprintf(FILE *restrict f, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

将格式化字符串输出到指定的 `FILE` 流 `f`。是 `vfprintf` 的可变参数包装器。

#### 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配

#### 后置条件

- Case 1 成功：返回写入 `f` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
fprintf(f, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vfprintf(f, fmt, ap) 委托核心引擎
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vfprintf()` — 格式化输出核心引擎（见 `vfprintf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏
