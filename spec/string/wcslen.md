# wcslen

## 函数签名
```c
size_t wcslen(const wchar_t *s);
```

## 意图
计算宽字符串 s 的长度（不含终止 L'\0'）。

## 前置条件
- `s != NULL`
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 返回值 == 满足 s[i] == L'\0' 的最小 i
- s 指向的字符串内容不变

## 不变量
- s 指针单调递增直到遇到 L'\0'

## 算法
逐字符遍历到 L'\0'，返回指针差值 s - a。

/* Rely */
[RELY]
Predefined Structures/Functions:
  (无外部函数依赖)

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  size_t wcslen(const wchar_t *s);  // 计算宽字符串长度
