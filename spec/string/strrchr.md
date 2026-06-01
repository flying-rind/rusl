# strrchr

## 函数签名
```c
char *strrchr(const char *s, int c);
```

## 意图
在字符串 s 中从后向前查找字符 c 最后一次出现的位置。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 c == '\0'，返回值为 &s[strlen(s)]（指向终止 null）
- 若 c != '\0' 且字符串中包含 c，返回值为 c 最后一次出现的位置
- 若 c != '\0' 且字符串中不包含 c，返回值为 NULL
- s 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 __memrchr(s, c, strlen(s) + 1)，在包括终止 null 在内的所有字符中从后向前搜索。

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *__memrchr(const void *, int, size_t);  // 依赖1: 从后向前查找字符，musl 内部函数
  size_t strlen(const char *);                  // 依赖2: 计算字符串长度，定义于 <string.h>

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strrchr(const char *s, int c);  // 本模块保证对外提供的接口签名
