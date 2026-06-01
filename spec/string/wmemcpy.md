# wmemcpy

## 函数签名
```c
wchar_t *wmemcpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

## 意图
将 s 指向的宽字符数组中的前 n 个元素复制到 d 指向的数组中。调用者保证 d 和 s 不重叠。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 至少可写入 n 个 wchar_t
- `s` 至少可读取 n 个 wchar_t

## 后置条件
- d[0..n-1] == s[0..n-1]
- 返回值为 d

## 不变量
- n 递减确保恰好复制 n 个元素

## 算法
简单的 while 循环逐元素复制，每次递增两个指针并递减 n。

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wmemcpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
