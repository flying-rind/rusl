# wcsnlen

## 函数签名
```c
size_t wcsnlen(const wchar_t *s, size_t n);
```

## 意图
计算宽字符串 s 的长度，最多搜索 n 个宽字符。

## 前置条件
- `s != NULL`
- 当 `n > 0` 时，s 指向至少可读取 min(n, wcslen(s)+1) 个宽字符的内存区域

## 后置条件
- 若 s 在 n 个宽字符内包含 L'\0'，返回值为 wcslen(s)
- 若 s 的前 n 个宽字符均非 L'\0'，返回值为 n
- s 指向的内存内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 wmemchr(s, 0, n) 在 n 个宽字符范围内搜索 L'\0'。若找到则返回偏移量，否则返回 n。

/* Rely */
[RELY]
Predefined Structures/Functions:
  wchar_t *wmemchr(const wchar_t *s, wchar_t c, size_t n);  // 依赖1: 在宽字符数组中搜索L'\0'

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  size_t wcsnlen(const wchar_t *s, size_t n);  // 限定长度的宽字符串长度计算
