# wcschr

## 函数签名
```c
wchar_t *wcschr(const wchar_t *s, wchar_t c);
```

## 意图
在宽字符串 s 中查找宽字符 c 第一次出现的位置。

## 前置条件
- `s != NULL`
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 若 c == L'\0'，返回值为 &s[wcslen(s)]（指向终止 null）
- 若 c != L'\0' 且 s 中不包含 c，返回值为 NULL
- 若 s 中包含 c，返回值为第一个匹配位置的指针
- s 指向的字符串内容不变

## 不变量
- s 指针单调递增，循环在找到 c 或到达 L'\0' 时终止

## 算法
1. 若 c == L'\0'，直接返回 s + wcslen(s)
2. 否则遍历 s，检查 *s == c，直到 L'\0'
3. 若找到匹配返回当前指针，否则返回 NULL

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcslen(const wchar_t *s);  // 依赖1: 宽字符串长度计算（当 c==L'\0' 时用于定位终止位置）

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcschr(const wchar_t *s, wchar_t c);  // 本模块导出的函数签名
```
