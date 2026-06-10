# fwscanf.c 规约

> musl libc 宽字符格式化文件流输入函数。是 `vfwscanf(f, ...)` 的可变参数包装。

---

## 依赖图

```
fwscanf (Public)
  └─> vfwscanf(f, fmt, ap)  (see vfwscanf.c spec)

__isoc99_fwscanf (weak_alias)
  └─> fwscanf
```

---

## 函数规约

### 1. fwscanf

```c
int fwscanf(FILE *restrict f, const wchar_t *restrict fmt, ...);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从 `FILE` 流 `f` 读取宽字符格式化输入。是 `vfwscanf` 的可变参数包装器。与 `fscanf` 的区别在于格式字符串为宽字符，且匹配的字符按宽字符处理。

#### 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF 或编码错误）：返回 `EOF`（即 `WEOF`）
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
fwscanf(f, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vfwscanf(f, fmt, ap) 委托核心引擎
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vfwscanf()` — 宽字符格式化输入核心引擎（见 `vfwscanf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

---

### 2. __isoc99_fwscanf (weak_alias)

```c
weak_alias(fwscanf, __isoc99_fwscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部 C99 兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_fwscanf` 弱别名。与 `fwscanf` 行为完全相同。
