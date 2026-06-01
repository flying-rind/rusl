# strncmp

## 函数签名
```c
int strncmp(const char *_l, const char *_r, size_t n);
```

## 意图
比较两个 C 字符串的前 n 个字符。

## 前置条件
- `_l != NULL`
- `_r != NULL`
- _l 和 _r 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 n == 0，返回值为 0
- 返回值 == 0：两个字符串的前 min(n, ...) 个字符对应相等，或在 n 个字符内同时到达 '\0'
- 返回值 < 0：在第一个不同字符处 *_l < *_r（作为 unsigned char）
- 返回值 > 0：在第一个不同字符处 *_l > *_r
- _l 和 _r 指向的字符串内容不变

## 不变量
- n 递减确保不比较超过 n 个字符
- 比较提前结束：任意字符为 '\0' 或 n==0 或字符不相等时停止

## 算法
逐字符比较，n 首先递减，循环中比较字符和 n。返回值为两字节作为 unsigned char 的差值。

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)  // 纯算法实现，无外部函数调用

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  int strncmp(const char *_l, const char *_r, size_t n);
