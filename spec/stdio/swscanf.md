# swscanf.c 规约

> musl libc 宽字符串格式化输入函数。是 `vswscanf(s, fmt, ...)` 的可变参数包装。

---

## 依赖图

```
swscanf (Public)
  └─> vswscanf(s, fmt, ap)  (see vswscanf.c spec)

__isoc99_swscanf (weak_alias)
  └─> swscanf
```

---

## 函数规约

### 1. swscanf

```c
int swscanf(const wchar_t *restrict s, const wchar_t *restrict fmt, ...);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从宽字符串 `s` 读取格式化输入。是 `vswscanf` 的可变参数包装器。与 `sscanf` 的区别在于输入字符串和格式字符串均为宽字符。

#### 前置条件

- `s != NULL`，指向以 `L'\0'` 结尾的有效宽字符串输入源
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串末尾）：返回 `EOF`
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
swscanf(s, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vswscanf(s, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vswscanf()` — `va_list` 版宽字符串格式化输入（见 `vswscanf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

---

### 2. __isoc99_swscanf (weak_alias)

```c
weak_alias(swscanf, __isoc99_swscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部 C99 兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_swscanf` 弱别名。与 `swscanf` 行为完全相同。
