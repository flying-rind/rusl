# dprintf.c 规约

> musl libc 文件描述符格式化输出函数。是 `vdprintf(fd, ...)` 的可变参数包装。

---

## 依赖图

```
dprintf
  └─> vdprintf(fd, fmt, ap)  (see vdprintf.c spec)
```

---

## 函数规约

### 1. dprintf

```c
int dprintf(int fd, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（POSIX 扩展）

#### Intent

将格式化字符串写入文件描述符 `fd`。是 `vdprintf` 的可变参数包装器。

#### 前置条件

- `fd` 为有效的文件描述符
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配

#### 后置条件

- Case 1 成功：返回写入 `fd` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
dprintf(fd, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vdprintf(fd, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vdprintf()` — `va_list` 版文件描述符格式化输出（见 `vdprintf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏
