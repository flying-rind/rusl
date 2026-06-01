# wmemchr

## 函数签名
```c
wchar_t *wmemchr(const wchar_t *s, wchar_t c, size_t n);
```

## 意图
在 s 指向的宽字符数组的前 n 个元素中查找宽字符 c 第一次出现的位置。

## 前置条件
- `s != NULL`
- 当 `n > 0` 时，`s` 指向的内存区域至少可读取 n 个 wchar_t

## 后置条件
- 若存在 i，0 <= i < n，使得 s[i] == c，返回值为 (wchar_t*)(s + i)
- 若不存在，返回值为 NULL
- s 指向的内存内容不变

## 不变量
- n 递减，确保不搜索超过 n 个元素

## 算法
逐元素遍历直到 n 耗尽或找到匹配的宽字符 c。

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wmemchr(const wchar_t *s, wchar_t c, size_t n);
