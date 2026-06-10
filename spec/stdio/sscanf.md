# sscanf.c 规约

> musl libc 字符串格式化输入函数。是 `vsscanf(s, ...)` 的可变参数包装。

---

## 依赖图

```
sscanf
  └─> vsscanf(s, fmt, ap)  (see vsscanf.c spec)
        └─> vfscanf(&f, fmt, ap)  (see vfscanf.c spec)

__isoc99_sscanf (weak_alias)
  └─> sscanf
```

---

## 函数规约

### 1. sscanf

```c
int sscanf(const char *restrict s, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

从内存中的 null 结尾字符串 `s` 读取格式化输入。是 `vsscanf` 的可变参数包装器。

#### 前置条件

- `s` 指向有效的 null 结尾 C 字符串
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串结尾）：返回 `EOF`
- `s` 源字符串不会被修改
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
sscanf(s, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vsscanf(s, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vsscanf()` — `va_list` 版字符串格式化输入（见 `vsscanf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

---

### 2. __isoc99_sscanf (weak_alias)

```c
weak_alias(sscanf, __isoc99_sscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_sscanf` 弱别名。与 `sscanf` 行为完全相同。
