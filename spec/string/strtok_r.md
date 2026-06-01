# strtok_r

## 函数签名
```c
char *strtok_r(char *restrict s, const char *restrict sep, char **restrict p);
```

## 意图
strtok 的可重入（线程安全）版本。从字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态。

## 前置条件
- `p != NULL`
- 首次调用时 `s != NULL`；后续调用可传 `s == NULL`
- `sep != NULL`
- sep 指向以 null 结尾的有效 C 字符串
- 当 `s != NULL` 时，s 指向可修改的以 null 结尾的有效 C 字符串

## 后置条件
- 若无更多 token，返回值为 NULL，且 *p 被设为 0（NULL）
- 若有 token：返回值为指向 token 起始位置的指针，token 末尾被 '\0' 替换
- *p 被更新为下一个搜索位置（或 NULL）

## 不变量
- *p 始终为 NULL（无更多 token）或指向下一个搜索起始位置

## 算法
1. 若 s == NULL，使用 *p 作为续接起始位置
2. 使用 strspn 跳过前导分隔符
3. 若到达末尾，设置 *p=0，返回 NULL
4. 使用 strcspn 找到分隔符位置，更新 *p
5. 若找到分隔符，用 '\0' 替代并递增 *p
6. 返回 token 起始位置

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strspn(const char *, const char *);   // 依赖1: 计算在前缀中的连续字符数，定义于 <string.h>
  size_t strcspn(const char *, const char *);  // 依赖2: 计算不在前缀中的连续字符数，定义于 <string.h>

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strtok_r(char *restrict s, const char *restrict sep, char **restrict p);  // 本模块保证对外提供的接口签名
