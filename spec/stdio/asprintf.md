# asprintf.c 规约

> musl libc 自动分配缓冲区的格式化输出函数。是 `vasprintf(s, ...)` 的可变参数包装（GNU 扩展）。

---

## 依赖图

```
asprintf
  └─> vasprintf(s, fmt, ap)  (see vasprintf.c spec)
```

---

## 函数规约

### 1. asprintf

```c
int asprintf(char **s, const char *fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（GNU 扩展 / POSIX）

#### Intent

将格式化字符串写入动态分配的缓冲区。缓冲区由 `malloc` 分配，调用者负责 `free`。是 `vasprintf` 的可变参数包装器。

#### 前置条件

- `s != NULL`，`*s` 的值将被覆盖
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配

#### 后置条件

- Case 1 成功：
  - `*s` 指向 `malloc` 分配的缓冲区，包含 null 结尾的格式化字符串
  - 返回值为格式化字符串的长度（不含 `'\0'`）
  - 调用者有责任 `free(*s)`
- Case 2 失败（格式错误或分配失败）：返回 `-1`，`*s` 不变
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
asprintf(s, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vasprintf(s, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vasprintf()` — `va_list` 版自动分配格式化输出（见 `vasprintf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏
