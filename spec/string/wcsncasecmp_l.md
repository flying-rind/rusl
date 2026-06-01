# wcsncasecmp_l

## 函数签名
```c
int wcsncasecmp_l(const wchar_t *l, const wchar_t *r, size_t n, locale_t locale);
```

## 意图
在指定 locale 下忽略大小写比较两个宽字符串的前 n 个宽字符。

## 前置条件
- `l != NULL`
- `r != NULL`
- l 和 r 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 与 wcsncasecmp(l, r, n) 返回相同的值
- l 和 r 指向的字符串内容不变

## 不变量
- locale 参数被忽略

## 算法
直接委托给 wcsncasecmp(l, r, n)，忽略 locale 参数。

/* Rely */
[RELY]
Predefined Structures/Functions:
  int wcsncasecmp(const wchar_t *l, const wchar_t *r, size_t n);  // 依赖1: 委托给无locale版本

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  int wcsncasecmp_l(const wchar_t *l, const wchar_t *r, size_t n, locale_t locale);  // 指定locale的宽字符串忽略大小写比较
