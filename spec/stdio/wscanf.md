# wscanf.c 规约

> musl libc 宽字符标准输入格式化函数。是 `vwscanf(fmt, ...)` 的可变参数包装，最终委托给 `vfwscanf(stdin, ...)`。

---

## 依赖图

```
wscanf (Public)
  └─> vwscanf(fmt, ap)  (see vwscanf.c spec)
        └─> vfwscanf(stdin, fmt, ap)  (see vfwscanf.c spec)

__isoc99_wscanf (weak_alias)
  └─> wscanf
```

---

## 函数规约

### 1. wscanf

```c
int wscanf(const wchar_t *restrict fmt, ...);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从标准输入流 `stdin` 读取宽字符格式化输入。是 `vwscanf` 的可变参数包装器。与 `scanf` 的区别在于格式字符串为宽字符，且匹配的字符按宽字符处理。

#### 前置条件

- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）
- `stdin` 已初始化，可读取

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
wscanf(fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vwscanf(fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vwscanf()` — `va_list` 版标准输入宽字符格式化读取（见 `vwscanf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

---

### 2. __isoc99_wscanf (weak_alias)

```c
weak_alias(wscanf, __isoc99_wscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部 C99 兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_wscanf` 弱别名。与 `wscanf` 行为完全相同。
