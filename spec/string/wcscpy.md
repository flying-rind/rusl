# wcscpy

## 函数签名
```c
wchar_t *wcscpy(wchar_t *restrict d, const wchar_t *restrict s);
```

## 意图
将 s 指向的宽字符串（包括终止 L'\0'）复制到 d。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 指向的缓冲区至少可写入 (wcslen(s) + 1) 个 wchar_t
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- d[0..len] == s[0..len]（其中 len == wcslen(s)），包括终止 L'\0'
- 返回值为 d

## 不变量
- 循环中的字符复制始终包含 L'\0'

## 算法
通过 `while ((*d++ = *s++))` 循环逐字符复制，包括终止 L'\0'。保存并返回原始 d 值 a。

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)  // 纯内联实现，不调用任何外部函数

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcscpy(wchar_t *restrict d, const wchar_t *restrict s);  // 本模块导出的函数签名
```
