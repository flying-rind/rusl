# wcsrchr

## 函数签名
```c
wchar_t *wcsrchr(const wchar_t *s, wchar_t c);
```

## 意图
在宽字符串 s 中从后向前查找宽字符 c 最后一次出现的位置。

## 前置条件
- `s != NULL`
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 若 s 中包含 c（包括 c == L'\0'），返回值为 c 最后一次出现的位置
- 若 s 中不包含 c，返回值为 NULL
- s 指向的字符串内容不变

## 不变量
- 指针 p 从字符串末尾向开头递减搜索

## 算法
从 s + wcslen(s)（即 L'\0' 位置）向回遍历到 s，返回第一个匹配 c 的位置。若遍历到 s 之前仍未找到，返回 NULL。

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcslen(const wchar_t *s);  // 依赖1: 获取宽字符串长度

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcsrchr(const wchar_t *s, wchar_t c);
