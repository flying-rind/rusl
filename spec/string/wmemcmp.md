# wmemcmp

## 函数签名
```c
int wmemcmp(const wchar_t *l, const wchar_t *r, size_t n);
```

## 意图
比较 l 和 r 指向的宽字符数组的前 n 个元素。

## 前置条件
- `l != NULL`
- `r != NULL`
- 当 `n > 0` 时，l 和 r 指向的内存区域各自至少可读取 n 个 wchar_t

## 后置条件
- 返回值 == 0：所有 n 个宽字符对应相等
- 返回值 == -1：在第一个不相等位置 l[i] < r[i]
- 返回值 == 1：在第一个不相等位置 l[i] > r[i]
- l 和 r 指向的内存内容不变

## 不变量
- n 递减确保不比较超过 n 个元素
- 比较在第一个不相等处停止

## 算法
逐宽字符比较直到 n 耗尽或字符不等。返回值限定为 -1、0 或 1。

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  int wmemcmp(const wchar_t *l, const wchar_t *r, size_t n);
