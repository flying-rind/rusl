# wcswcs

## 函数签名
```c
wchar_t *wcswcs(const wchar_t *haystack, const wchar_t *needle);
```

## 意图
在宽字符串 haystack 中查找子串 needle 第一次出现的位置。该函数是 wcsstr 的 BSD 别名。

## 前置条件
- `haystack != NULL`
- `needle != NULL`
- haystack 和 needle 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 与 wcsstr(haystack, needle) 完全相同的语义
- haystack 和 needle 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 wcsstr(haystack, needle)。

/* Rely */
[RELY]
Predefined Structures/Functions:
  wchar_t *wcsstr(const wchar_t *h, const wchar_t *n);  // 依赖1: 委托给wcsstr

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcswcs(const wchar_t *haystack, const wchar_t *needle);
