# putwchar.c 规约

> musl libc 标准输出宽字符写入函数。将一个宽字符写入 `stdout`。

---

## 依赖图

```
putwchar (Public)
  └─> fputwc(c, stdout)  (see fputwc.c spec)

putwchar_unlocked (weak_alias)
  └─> putwchar

stdout (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### 1. putwchar

```c
wint_t putwchar(wchar_t c);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

将宽字符 `c` 写入标准输出流 `stdout`。等价于 `putwc(c, stdout)` 或 `fputwc(c, stdout)`。

#### 前置条件

- `c`: 要写入的宽字符（`wchar_t` 类型）
- `stdout` 已正确初始化并处于可写状态

#### 后置条件

- **Case 1 成功写入宽字符**
  - 返回写入的宽字符值 `c`
  - 宽字符已转换为多字节序列并写入 `stdout`

- **Case 2 写入错误或编码错误**
  - 返回 `WEOF`
  - `stdout` 设置 `F_ERR` 标志

#### 系统算法

```
putwchar(c):
  return fputwc(c, stdout)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `fputwc(wchar_t, FILE *)` — 宽字符写入函数（见 `fputwc.c`）
- `stdout` — 标准输出 FILE 指针（`<stdio.h>` 全局变量）

---

### 2. putwchar_unlocked (weak_alias)

```c
weak_alias(putwchar, putwchar_unlocked);
```

[Visibility]: User — POSIX 免锁扩展，通过 `<wchar.h>` 对外导出

- **Intention**: 提供免锁版本的 `putwchar`。在 musl 中 `putwchar` 内部通过 `fputwc` 加锁，而 `putwchar_unlocked` 作为弱别名指向同一实现。实际行为与 `putwchar` 相同。

前置/后置条件及行为：完全等同于 `putwchar`。
