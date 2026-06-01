# strcasestr

## 函数签名
```c
char *strcasestr(const char *h, const char *n);
```

## 意图
在字符串 h（haystack）中忽略大小写查找子串 n（needle）第一次出现的位置。

## 前置条件
- `h != NULL`
- `n != NULL`
- h 和 n 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 strlen(n) == 0，返回 (char*)h
- 若在 h 中找到与 n 忽略大小写匹配的子串，返回指向 h 中第一个匹配位置的指针
- 若未找到，返回 NULL
- h 和 n 指向的字符串内容不变

## 不变量
- 外层循环遍历 h 的每个位置，内层通过 strncasecmp 比较 len=n 个字符
- 每次外层迭代 h 递增 1，直至 *h 为 '\0'

## 算法
1. 计算 n 的长度 l
2. 若 l==0，返回 h
3. 遍历 h：对每个起始位置 i，调用 strncasecmp(h+i, n, l)
4. 若匹配，返回 h+i
5. 遍历结束未找到则返回 0

注意：该算法是 O(|h|*|n|) 的朴素算法。

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strlen(const char *s);              // 依赖1: 计算 needle 字符串的长度 l
  int strncasecmp(const char *l, const char *r, size_t n);  // 依赖2: 忽略大小写比较前 n 个字符
  size_t;             // 依赖3: 标准无符号整数类型，用于字符串长度
  char *;             // 依赖4: 返回值类型
  NULL;               // 依赖5: 空指针常量，未找到时返回值

Predefined Macros:
  _GNU_SOURCE;        // 特性测试宏: 启用 GNU 扩展函数 strcasestr 的定义

[GUARANTEE]
Exported Interface:
  char *strcasestr(const char *h, const char *n);  // 在 h 中忽略大小写查找子串 n 第一次出现的位置
