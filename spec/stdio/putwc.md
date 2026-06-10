# putwc.c 规约

> musl libc 宽字符输出函数。等价于 `fputwc(c, f)`，直接向 FILE 流写入一个宽字符。

---

## 依赖图

```
putwc (Public)
  └─> fputwc(c, f)  (see fputwc.c spec)
```

---

## 函数规约

### 1. putwc

```c
wint_t putwc(wchar_t c, FILE *f);
```

[Visibility]: User — `<wchar.h>` 或 `<stdio.h>` 标准库函数，用户程序可直接调用。通常由宏实现，但 musl 同时提供函数实现以支持函数指针调用等场景。

#### Intent

将宽字符 `c` 写入 FILE 流 `f`。等价于 `fputwc(c, f)`。与 `fputwc` 唯一区别在于某些实现中 `putwc` 可作为宏内联展开，但在 musl 中两者实现完全相同。

#### 前置条件

- `c`: 要写入的宽字符（`wchar_t` 类型）
- `f`: 非空 FILE 指针，指向已打开的写模式流
- 流的方向必须为宽字符模式（若尚未设置，`fputwc` 内部会调用 `fwide(f, 1)` 设置）

#### 后置条件

- **Case 1 成功写入宽字符**
  - 返回写入的宽字符值 `c`
  - 宽字符已转换为多字节序列并写入流

- **Case 2 写入错误或编码错误**
  - 返回 `WEOF`
  - FILE 流设置 `F_ERR` 标志

#### 系统算法

```
putwc(c, f):
  return fputwc(c, f)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `fputwc(wchar_t, FILE *)` — 宽字符写入函数（见 `fputwc.c`）
