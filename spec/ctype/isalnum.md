# isalnum.c 规约

> musl libc C 标准库函数 `isalnum`/`isalnum_l` 实现。`isalnum` 检查字符是否为字母或数字（等价于 `isalpha(c) || isdigit(c)`）。

---

## 依赖图

```
isalnum
  ├─> isalpha  (see isalpha.c spec)
  └─> isdigit  (see isdigit.c spec)
```

---

## 函数规约

### 1. isalnum

```c
int isalnum(int c);
```

[Visibility]: Public — ISO C 标准函数，定义于 `<ctype.h>`

#### Intent

检查字符 `c` 是否为字母或数字字符。字母包括 `A-Z`、`a-z` 以及 locale 定义的其他字母字符；数字包括 `0-9`。

#### 前置条件

- `c` 为 `int` 类型，其值必须能表示为 `unsigned char` 或等于 `EOF`
- 若 `c` 的值不在 `unsigned char` 范围内且不等于 `EOF`，行为未定义

#### 后置条件

- 若 `c` 是字母（`isalpha(c)` 为真）或数字（`isdigit(c)` 为真），返回非零值
- 否则返回 `0`
- 不设置 `errno`

#### 系统算法

```
isalnum(c):
  return isalpha(c) || isdigit(c)
```

#### 不变量

- 对于所有 `c`，`isalnum(c)` ≡ `isalpha(c) || isdigit(c)`
- 对于 `C` locale：`isalnum(c)` 为真当且仅当 `c ∈ [A-Z] ∪ [a-z] ∪ [0-9]`

---

### 2. \_\_isalnum_l (internal)

```c
int __isalnum_l(int c, locale_t l);
```

[Visibility]: Internal — musl 内部 locale-aware 实现

#### Intent

`isalnum` 的 locale-aware 版本。在 musl 中，当前实现忽略 `l` 参数，直接委托给 `isalnum(c)`。

#### 前置条件

- 同 `isalnum`
- `l` 为有效的 `locale_t` 句柄

#### 后置条件

- 返回值等价于 `isalnum(c)`

---

### 3. isalnum_l (weak_alias)

```c
weak_alias(__isalnum_l, isalnum_l);
```

[Visibility]: Public — POSIX.1-2008 扩展，定义于 `<ctype.h>`

- **Intention**: 通过弱别名将 `__isalnum_l` 暴露为 POSIX 标准函数 `isalnum_l`。

前置/后置条件及行为：完全等同于 `__isalnum_l`。
