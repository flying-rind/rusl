# strtok

## 函数签名
```c
char *strtok(char *restrict s, const char *restrict sep);
```

## 意图
从字符串 s 中提取下一个 token，token 之间由 sep 中的任意字符分隔。使用静态内部指针维护状态，因此非线程安全。

## 前置条件
- 首次调用时 `s != NULL`；后续调用可传 `s == NULL` 继续处理同一字符串
- `sep != NULL`
- sep 指向以 null 结尾的有效 C 字符串（不可为 NULL，不可在 token 化过程中改变）
- 当 `s != NULL` 时，s 指向可修改的以 null 结尾的有效 C 字符串

## 后置条件
- 若无更多 token，返回值为 NULL
- 若有 token：返回值为指向 token 起始位置的指针，且 token 末尾被 '\0' 替换
- 内部静态指针被更新为下一个搜索位置

## 不变量
- 静态指针 p 始终为 NULL（无更多 token）或指向下一个搜索起始位置
- 分隔符集合 sep 在多次调用间应保持一致

## 算法
1. 若 s == NULL，使用静态变量 p 作为续接起始位置
2. 使用 strspn 跳过前导分隔符
3. 若到达字符串末尾，设置 p=NULL 并返回 NULL
4. 使用 strcspn 找到下一个分隔符位置
5. 若找到分隔符，用 '\0' 替代，更新 p 为分隔符后的位置
6. 若未找到分隔符，设置 p=NULL
7. 返回 token 起始位置

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strspn(const char *, const char *);   // 依赖1: 计算在前缀中的连续字符数，定义于 <string.h>
  size_t strcspn(const char *, const char *);  // 依赖2: 计算不在前缀中的连续字符数，定义于 <string.h>

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strtok(char *restrict s, const char *restrict sep);  // 本模块保证对外提供的接口签名
