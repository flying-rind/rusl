# vwscanf.c 规约

> musl libc 宽字符标准输入格式化函数（va_list 版本）。直接委托给 `vfwscanf(stdin, ...)`。

---

## 依赖图

```
vwscanf (Public)
  └─> vfwscanf(stdin, fmt, ap)  (see vfwscanf.c spec)

__isoc99_vwscanf (weak_alias)
  └─> vwscanf

stdin (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### 1. vwscanf

```c
int vwscanf(const wchar_t *restrict fmt, va_list ap);
```

[Visibility]: User — `<stdarg.h>` / `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从标准输入流 `stdin` 读取宽字符格式化输入。是 `wscanf` 的 `va_list` 版本。直接委托给 `vfwscanf(stdin, fmt, ap)`。

#### 前置条件

- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化
- `stdin` 已初始化，可读取

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数

#### 系统算法

```
vwscanf(fmt, ap):
  return vfwscanf(stdin, fmt, ap)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vfwscanf()` — 宽字符格式化输入核心引擎（见 `vfwscanf.c`）
- `stdin` — 标准输入流（见 `src/stdio/__stdin_used.c`）

---

### 2. __isoc99_vwscanf (weak_alias)

```c
weak_alias(vwscanf, __isoc99_vwscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部 C99 兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_vwscanf` 弱别名。与 `vwscanf` 行为完全相同。
