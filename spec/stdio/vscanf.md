# vscanf.c 规约

> musl libc `va_list` 版标准输入格式化读取函数。直接委托 `vfscanf(stdin, ...)`。

---

## 依赖图

```
vscanf
  └─> vfscanf(stdin, fmt, ap)  (see vfscanf.c spec)

__isoc99_vscanf (weak_alias)
  └─> vscanf
```

---

## 函数规约

### 1. vscanf

```c
int vscanf(const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

从标准输入流 `stdin` 读取格式化输入（`va_list` 版本）。是 `scanf` 的 `va_list` 平替。

#### 前置条件

- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化
- `stdin` 已初始化，可读取

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数

#### 系统算法

```
vscanf(fmt, ap):
  return vfscanf(stdin, fmt, ap)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vfscanf()` — 格式化输入核心引擎（见 `vfscanf.c`）
- `stdin` — 标准输入流（见 `src/stdio/__stdin_used.c`）

---

### 2. __isoc99_vscanf (weak_alias)

```c
weak_alias(vscanf, __isoc99_vscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_vscanf` 弱别名。与 `vscanf` 行为完全相同。
