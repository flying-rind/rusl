# rindex

## 函数签名
```c
char *rindex(const char *s, int c);
```

## 意图
在字符串 s 中从后向前查找字符 c 最后一次出现的位置。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 如果存在 i >= 0 使得 s[i] == (char)c 且对所有 j > i，s[j] != c（直到 '\0'），返回值为 &s[i]
- 如果不存在，返回值为 NULL
- s 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 strrchr(s, c)，语义完全等价。

/* Rely */
[RELY]
Predefined Structures/Functions:
  char *strrchr(const char *s, int c);  // 依赖1: 完成实际的从后向前字符查找
  char *;             // 依赖2: 返回值类型
  int;                // 依赖3: 参数 c 的类型
  NULL;               // 依赖4: 空指针常量，未找到时返回值

Predefined Macros:
  _BSD_SOURCE;        // 特性测试宏: 启用 BSD 兼容函数 rindex 的定义

[GUARANTEE]
Exported Interface:
  char *rindex(const char *s, int c);  // BSD 兼容的从后向前字符查找 (等价于 strrchr)
