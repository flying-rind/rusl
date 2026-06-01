# wcsncpy

## 函数签名
```c
wchar_t *wcsncpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

## 意图
将 s 中最多 n 个宽字符复制到 d。若 wcslen(s) < n，用 L'\0' 填充剩余位置。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 指向的缓冲区至少可写入 n 个 wchar_t
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- d[0..min(len, n)-1] == s[0..min(len, n)-1]（其中 len == wcslen(s)）
- 若 len < n，d[len..n-1] 全部为 L'\0'
- 返回值为 d

## 不变量
- 复制停止于 n 耗尽或遇到 L'\0'

## 算法
1. 循环复制非 L'\0' 宽字符，同时递减 n
2. 使用 wmemset 将剩余 n 个位置填充为 L'\0'
3. 返回保存的原始 d 值 a

/* Rely */
[RELY]
Predefined Structures/Functions:
  wchar_t *wmemset(wchar_t *d, wchar_t c, size_t n);  // 依赖1: 用L'\0'填充剩余位置

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  wchar_t *wcsncpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);  // 复制最多n个宽字符，不足补L'\0'
