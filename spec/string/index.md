# index

## 函数签名
```c
char *index(const char *s, int c);
```

## 意图
在字符串 s 中查找字符 c 第一次出现的位置。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 如果存在 i >= 0 使得 s[i] == (char)c，则返回值为 &s[i]
- 如果不存在，返回值为 NULL
- s 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 strchr(s, c)，语义完全等价。

/* Rely */
[RELY]
Predefined Structures/Functions:
  char *strchr(const char *s, int c);  // 依赖1: 在字符串中查找字符首次出现位置

Predefined Macros:
  _BSD_SOURCE  // 依赖2: 启用 BSD 兼容函数声明（index 为 BSD 扩展）

[GUARANTEE]
Exported Interface:
  char *index(const char *s, int c);  // 在字符串 s 中查找字符 c 第一次出现的位置
