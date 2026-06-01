# strcspn

## 函数签名
```c
size_t strcspn(const char *s, const char *c);
```

## 意图
计算 s 的起始段长度，该段中不包含字符串 c 中的任何字符。即返回 s 中第一个匹配 c 中任意字符的位置索引。

## 前置条件
- `s != NULL`
- `c != NULL`
- s 和 c 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == min{ i >= 0 | s[i] == '\0' 或 s[i] 属于 c } 的 i 值
- s 和 c 指向的字符串内容不变

## 不变量
- s 指针单调递增，最终定位在第一个匹配字符或 '\0' 处

## 算法
1. 若 c 为空字符串或仅包含一个字符：使用 __strchrnul(s, *c) 快速计算偏移量
2. 一般情况：构建 256 位（32 字节）的位图 byteset，将 c 中所有字符位置为 1
3. 扫描 s：对每个字符检查其在 byteset 中的位，直到命中或遇到 '\0'

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 长度计算和位图索引类型
  unsigned char;  // 依赖2: 位图操作时的字符类型转换
  __strchrnul(const char *s, int c);  // 依赖3: c 为单个或空字符时快速计算跨度
  memset(void *s, int c, size_t n);  // 依赖4: 清零位图 byteset

Predefined Macros:
  BITOP(a,b,op);  // 内部宏: 对字节数组 bitset 执行位操作

[GUARANTEE]
Exported Interface:
  size_t strcspn(const char *s, const char *c);  // 计算 s 中不含 c 中任何字符的前缀长度
