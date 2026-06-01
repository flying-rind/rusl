# wcsncmp

## 函数签名
```c
int wcsncmp(const wchar_t *l, const wchar_t *r, size_t n);
```

## 意图
比较两个宽字符串的前 n 个宽字符。

## 前置条件
- `l != NULL`
- `r != NULL`
- l 和 r 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 返回值 == 0：前 n 个宽字符相等，或 n == 0，或在 n 个字符内同时到达 L'\0'
- 返回值 == -1：l 字典序小于 r
- 返回值 == 1：l 字典序大于 r
- l 和 r 指向的字符串内容不变

## 不变量
- n 递减确保不比较超过 n 个宽字符
- 循环终止条件：n==0、*l != *r 或 *l == L'\0'

## 算法
逐宽字符比较直到 n 耗尽或字符不等或遇到 L'\0'。返回值限定为 -1、0 或 1。

/* Rely */
[RELY]
Predefined Structures/Functions:
  (无外部函数依赖)

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  int wcsncmp(const wchar_t *l, const wchar_t *r, size_t n);  // 比较宽字符串前n个字符
