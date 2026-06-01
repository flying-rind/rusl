# strchrnul

## 函数签名
```c
char *strchrnul(const char *s, int c);
```

## 意图
在字符串 s 中查找字符 c 第一次出现的位置。若未找到 c，返回指向终止 null 字符的指针（而非 NULL）。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若存在 i >= 0 使得 s[i] == (unsigned char)c，返回值为 &s[i]
- 若不存在，返回值为指向 s 末尾 '\0' 的指针
- s 指向的字符串内容不变

## 不变量
- search 位置始终在 s[0..strlen(s)] 区间内，包含终止 null
- 字级别搜索时 HASZERO 检测同时覆盖 '\0' 和目标字符 c

## 算法
1. 若 c == '\0'，直接返回 s + strlen(s)
2. 逐字节对齐到字边界，同时检查 '\0' 和目标字符
3. 使用 HASZERO(*w) 和 HASZERO(*w ^ k) 进行字级别并行检测
4. 逐字节精确定位目标字符或 '\0'

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: ALIGN 定义和 strlen 返回值类型
  uintptr_t;  // 依赖2: 地址对齐检测
  unsigned char;  // 依赖3: 字符比较时的类型转换
  strlen(const char *s);  // 依赖4: c == '\0' 时快速返回字符串末尾

Predefined Macros:
  __GNUC__;  // 依赖1: 编译器特性检测，使能字级别并行搜索优化
  UCHAR_MAX;  // 依赖2: 位于 <limits.h>，ONES 和 HIGHS 计算的基础常量
  weak_alias(old, new);  // 依赖3: 内部宏，用于创建弱符号别名

[GUARANTEE]
Exported Interface:
  char *__strchrnul(const char *s, int c);  // 查找字符串中字符 c，未找到则返回末尾 null 指针
  char *strchrnul(const char *s, int c);  // __strchrnul 的弱别名，公开接口
