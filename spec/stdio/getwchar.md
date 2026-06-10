# getwchar.c 规约

> musl libc 标准输入宽字符读取函数。从 `stdin` 读取一个宽字符。

---

## 依赖图

```
getwchar (Public)
  └─> fgetwc(stdin)  (see fgetwc.c spec)

getwchar_unlocked (weak_alias)
  └─> getwchar

stdin (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### 1. getwchar

```c
wint_t getwchar(void);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从标准输入流 `stdin` 读取一个宽字符。等价于 `getwc(stdin)` 或 `fgetwc(stdin)`。

#### 前置条件

- `stdin` 已正确初始化并处于可读状态

#### 后置条件

- **Case 1 成功读取宽字符**
  - 返回读取到的宽字符（`wchar_t` 类型的 `wint_t` 值）
  - `stdin` 流位置前进

- **Case 2 到达文件末尾**
  - 返回 `WEOF`
  - `stdin` 设置 `F_EOF` 标志

- **Case 3 读取错误或编码错误**
  - 返回 `WEOF`
  - `stdin` 设置 `F_ERR` 标志
  - 编码错误时设置 `errno = EILSEQ`

#### 系统算法

```
getwchar():
  return fgetwc(stdin)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `fgetwc(FILE *)` — 宽字符读取函数（见 `fgetwc.c`）
- `stdin` — 标准输入 FILE 指针（`<stdio.h>` 全局变量）

---

### 2. getwchar_unlocked (weak_alias)

```c
weak_alias(getwchar, getwchar_unlocked);
```

[Visibility]: User — POSIX 免锁扩展，通过 `<wchar.h>` 对外导出

- **Intention**: 提供免锁版本的 `getwchar`。在 musl 中 `getwchar` 内部通过 `fgetwc` 加锁，而 `getwchar_unlocked` 作为弱别名指向同一实现。实际行为与 `getwchar` 相同。

前置/后置条件及行为：完全等同于 `getwchar`。
