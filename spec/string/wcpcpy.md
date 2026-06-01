# wcpcpy

## 函数签名
```c
wchar_t *wcpcpy(wchar_t *restrict d, const wchar_t *restrict s);
```

## 意图
将 s 指向的宽字符串（包括终止 L'\0'）复制到 d，返回 d 中终止 null 的位置。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 指向的缓冲区至少可写入 (wcslen(s) + 1) 个 wchar_t
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- d[0..len] == s[0..len]（其中 len == wcslen(s)），包括终止 null
- 返回值为 &d[len]（指向终止 L'\0'）

## 不变量
- 无全局或静态状态被修改

## 算法
将 wcscpy 和 wcslen 组合：wcscpy(d, s) 复制字符串并返回 d，加上 wcslen(s) 得到终止 null 位置的指针。

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  wchar_t *wcscpy(wchar_t *restrict d, const wchar_t *restrict s);  // 依赖1: 宽字符串复制
  size_t wcslen(const wchar_t *s);                                   // 依赖2: 宽字符串长度计算

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcpcpy(wchar_t *restrict d, const wchar_t *restrict s);  // 本模块导出的函数签名
```
