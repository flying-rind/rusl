# wcsncasecmp

## 函数签名
```c
int wcsncasecmp(const wchar_t *l, const wchar_t *r, size_t n);
```

## 意图
在忽略大小写的情况下比较两个宽字符串的前 n 个宽字符。

## 前置条件
- `l != NULL`
- `r != NULL`
- l 和 r 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 若 n == 0，返回值为 0
- 返回值 == 0：前 min(n, ...) 个字符忽略大小写后相等
- 返回值 < 0：在第一个不同字符处 towlower(*l) < towlower(*r)
- 返回值 > 0：在第一个不同字符处 towlower(*l) > towlower(*r)
- l 和 r 指向的字符串内容不变

## 不变量
- n 递减确保不比较超过 n 个字符
- 循环终止条件：n==0、任一字符为 L'\0'、或转换为小写后不相等

## 算法
逐宽字符比较，通过 towlower() 转换为小写后比较。n 初始递减一次，循环中继续递减。

/* Rely */
[RELY]
Predefined Structures/Functions:
  wint_t towlower(wint_t wc);  // 依赖1: 将宽字符转换为小写

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  int wcsncasecmp(const wchar_t *l, const wchar_t *r, size_t n);  // 忽略大小写比较宽字符串前n个字符
