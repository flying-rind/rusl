# vasprintf.c 规约

> musl libc `va_list` 版自动分配缓冲区的格式化输出函数。先计算所需空间，再 `malloc` 分配缓冲区完成写入。

---

## 依赖图

```
vasprintf
  ├─> vsnprintf(0, 0, fmt, ap2)  (see vsnprintf.c spec) — Phase 1: 计算长度
  ├─> malloc(l+1)                 (see <stdlib.h>)      — 分配缓冲区
  └─> vsnprintf(*s, l+1, fmt, ap) (see vsnprintf.c spec) — Phase 2: 写入
```

---

## 函数规约

### 1. vasprintf

```c
int vasprintf(char **s, const char *fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（GNU 扩展 / POSIX）

#### Intent

将格式化字符串写入动态分配的缓冲区。缓冲区由 `malloc` 分配，调用者负责 `free`。采用两阶段策略：先干跑计算长度，再分配并写入。

#### 前置条件

- `s != NULL`，`*s` 的值将被覆盖（不要求有效）
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：
  - `*s` 指向 `malloc` 分配的缓冲区，包含格式化后的 null 结尾字符串
  - 返回值为格式化字符串的长度（不含 `'\0'`）
  - 调用者有责任 `free(*s)`
- Case 2 长度计算失败（编码错误等）：返回 `-1`，`*s` 不变
- Case 3 `malloc` 失败：返回 `-1`，`*s` 不变

#### 系统算法

```
vasprintf(s, fmt, ap):
  1. va_copy(ap2, ap) 复制 va_list
  2. l = vsnprintf(NULL, 0, fmt, ap2)  // Phase 1: 仅计算长度
     va_end(ap2)
  3. if l < 0: return -1                // 编码错误
  4. *s = malloc(l + 1)                 // 分配缓冲区 (+1 for '\0')
  5. if *s == NULL: return -1           // 分配失败
  6. return vsnprintf(*s, l + 1, fmt, ap)  // Phase 2: 写入
```

#### 不变量

- 若 `vsnprintf(NULL, 0, ...)` 成功，其返回值精确等于"若缓冲区足够大时本应写入的字符数"
- `malloc(l+1)` 分配的缓冲区一定能容纳完整的格式化结果（包括 `'\0'`）

#### 依赖

- `vsnprintf()` — 有边界检查的格式化输出（见 `vsnprintf.c`）
- `malloc()` — 动态内存分配（见 `src/malloc/malloc.c`）
- `va_copy` / `va_end` — C99 可变参数操作宏
