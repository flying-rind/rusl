# strspn

## 函数签名
```c
size_t strspn(const char *s, const char *c);
```

## 意图
计算 s 的起始段长度，该段中的所有字符都属于集合 c。即返回 s 中第一个不属于 c 中字符的位置索引。

## 前置条件
- `s != NULL`
- `c != NULL`
- s 和 c 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == min{ i >= 0 | s[i] == '\0' 或 s[i] 不属于 c } 中 i 的值
- s 和 c 指向的字符串内容不变

## 不变量
- s 指针单调递增，位置从不越界

## 算法
1. 若 c 为空字符串，返回 0
2. 若 c 仅有一个字符，直接逐字符与 *c 比较
3. 一般情况：构建 256 位位图 byteset，将 c 中所有字符位置为 1
4. 扫描 s：对每个字符通过位图检查是否在 c 中，直到不匹配或遇到 '\0'

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;                                          // 依赖1: 无符号整数类型，定义于 <stddef.h>

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  size_t strspn(const char *s, const char *c);  // 本模块保证对外提供的接口签名
