# strncasecmp

## 函数签名
```c
int strncasecmp(const char *_l, const char *_r, size_t n);
```

## 意图
在忽略大小写的情况下比较两个 C 字符串的前 n 个字符。

## 前置条件
- `_l != NULL`
- `_r != NULL`
- _l 和 _r 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 n == 0，返回值为 0
- 返回值 == 0：两个字符串的前 min(n, min(strlen(_l), strlen(_r))) 个字符忽略大小写后相等
- 返回值 < 0：在第一个不同字符处 tolower(_l[i]) < tolower(_r[i])
- 返回值 > 0：在第一个不同字符处 tolower(_l[i]) > tolower(_r[i])
- _l 和 _r 指向的字符串内容不变

## 不变量
- n 递减，确保不比较超过 n 个字符
- 循环终止条件：n==0、任一字符为 '\0'、或字符（忽略大小写后）不相等

## 算法
逐字符比较，每个字符经 tolower() 转换后比较。n 初始递减一次（实现 n==0 的快速判断），循环中同时递减。返回值是两个小写字符的差值。

/* Rely */
[RELY]
Predefined Structures/Functions:
  int tolower(int c);                      // 依赖1: 将字符转换为小写，用于忽略大小写的比较
  locale_t;                                // 依赖2: 区域设置类型，用于__strncasecmp_l接口

Predefined Macros:
  weak_alias                               // musl内部宏，用于创建strncasecmp_l弱别名

[GUARANTEE]
Exported Interface:
  int strncasecmp(const char *_l, const char *_r, size_t n);
  int __strncasecmp_l(const char *l, const char *r, size_t n, locale_t loc);
  // weak_alias 提供: int strncasecmp_l(const char *l, const char *r, size_t n, locale_t loc);
