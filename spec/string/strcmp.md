# strcmp

## 函数签名
```c
int strcmp(const char *l, const char *r);
```

## 意图
比较两个 C 字符串 l 和 r 的字典序大小。

## 前置条件
- `l != NULL`
- `r != NULL`
- l 和 r 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == 0：两个字符串完全相等
- 返回值 < 0：在第一个不同字符处 l[i] < r[i]（作为 unsigned char 比较）
- 返回值 > 0：在第一个不同字符处 l[i] > r[i]（作为 unsigned char 比较）
- l 和 r 指向的字符串内容不变

## 不变量
- l 和 r 指针增量始终一致
- 循环在第一个不同字符处或同时到达 null 时终止

## 算法
逐字符比较 *l 和 *r，循环继续条件为 *l == *r 且 *l != '\0'（隐式地同时检查了 *r != '\0'）。返回值为两字节作为 unsigned char 的差值。

/* Rely */
[RELY]
Predefined Structures/Functions:
  unsigned char;  // 依赖1: 字符比较时使用的无符号类型

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  int strcmp(const char *l, const char *r);  // 字典序比较两个字符串
