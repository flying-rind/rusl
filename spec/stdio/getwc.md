# getwc.c 规约

> musl libc 宽字符输入函数。等价于 `fgetwc(f)`，直接从 FILE 流读取一个宽字符。

---

## 依赖图

```
getwc (Public)
  └─> fgetwc(f)  (see fgetwc.c spec)
```

---

## 函数规约

### 1. getwc

```c
wint_t getwc(FILE *f);
```

[Visibility]: User — `<wchar.h>` 或 `<stdio.h>` 标准库函数，用户程序可直接调用。通常由宏实现，但 musl 同时提供函数实现以支持函数指针调用等场景。

#### Intent

从 FILE 流 `f` 中读取一个宽字符。等价于 `fgetwc(f)`。与 `fgetwc` 唯一区别在于某些实现中 `getwc` 可作为宏内联展开以获得性能优化，但在 musl 中两者实现完全相同。

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的流
- 流的方向必须为宽字符模式（若尚未设置，`fgetwc` 内部会调用 `fwide(f, 1)` 设置）

#### 后置条件

- **Case 1 成功读取宽字符**
  - 返回读取到的宽字符（`wchar_t` 类型的 `wint_t` 值）
  - FILE 流位置前进

- **Case 2 到达文件末尾**
  - 返回 `WEOF`
  - FILE 流设置 `F_EOF` 标志

- **Case 3 读取错误或编码错误**
  - 返回 `WEOF`
  - FILE 流设置 `F_ERR` 标志
  - 编码错误时设置 `errno = EILSEQ`

#### 系统算法

```
getwc(f):
  return fgetwc(f)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `fgetwc(FILE *)` — 宽字符读取函数（见 `fgetwc.c`）
