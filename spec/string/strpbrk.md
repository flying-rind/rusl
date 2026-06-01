# strpbrk

## 函数签名
```c
char *strpbrk(const char *s, const char *b);
```

## 意图
在字符串 s 中查找 b 中任意字符第一次出现的位置。

## 前置条件
- `s != NULL`
- `b != NULL`
- s 和 b 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若存在 i >= 0 使得 s[i] != '\0' 且 s[i] 属于 b 中的某个字符，返回值为 &s[i]
- 若不存在（s 中的所有字符均不在 b 中），返回值为 NULL
- s 和 b 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
1. 使用 strcspn(s, b) 计算 s 中第一个 b 成员字符的索引
2. 若该位置的字符非 '\0'（即确实找到匹配字符），返回该位置
3. 否则返回 NULL

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strcspn(const char *s, const char *b);  // 依赖1: 计算s的前缀中不包含b中任一字符的长度

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strpbrk(const char *s, const char *b);
