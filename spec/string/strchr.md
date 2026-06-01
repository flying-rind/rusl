# strchr

## 函数签名
```c
char *strchr(const char *s, int c);
```

## 意图
在字符串 s 中查找字符 c 第一次出现的位置。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若存在 i >= 0 使得 s[i] == (char)c（且该字符包含 '\0'），返回值为 &s[i]
- 若不存在（即 c 非 '\0' 且字符串中不包含 c），返回值为 NULL
- s 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
调用 __strchrnul(s, c)，若返回地址处的字节等于 c 则返回该地址，否则返回 NULL。

/* Rely */
[RELY]
Predefined Structures/Functions:
  unsigned char;  // 依赖1: 比较时使用的无符号字符类型
  __strchrnul(const char *s, int c);  // 依赖2: 查找字符或终止 null 指针

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strchr(const char *s, int c);  // 在字符串 s 中查找字符 c 首次出现位置
