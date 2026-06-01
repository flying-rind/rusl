# wcpncpy

## 函数签名
```c
wchar_t *wcpncpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

## 意图
将 s 中最多 n 个宽字符复制到 d。若 s 长度小于 n，用 L'\0' 填充剩余位置。返回 d + min(wcslen(s), n)。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 指向的缓冲区至少可写入 n 个 wchar_t
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- d[0..min(len, n)-1] == s[0..min(len, n)-1]（其中 len == wcslen(s)）
- 若 len < n，d[len..n-1] 全部为 L'\0'
- 返回值为 d + min(len, n)

## 不变量
- 无全局或静态状态被修改

## 算法
组合 wcsncpy(d, s, n) 和 wcsnlen(s, n)：返回复制结束位置。

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  wchar_t *wcsncpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);  // 依赖1: 定长宽字符串复制
  size_t wcsnlen(const wchar_t *s, size_t n);                                    // 依赖2: 定长宽字符串长度计算

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcpncpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);  // 本模块导出的函数签名
```
