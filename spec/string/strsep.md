# strsep

## 函数签名
```c
char *strsep(char **str, const char *sep);
```

## 意图
从字符串 *str 中提取下一个 token，token 之间由 sep 中的任意字符分隔。与 strtok 不同，strsep 可以正确处理空 token。

## 前置条件
- `str != NULL`
- `sep != NULL`
- sep 指向以 null 结尾的有效 C 字符串
- 若 *str 非 NULL，*str 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 *str == NULL，返回值为 NULL（无更多 token）
- 若 *str 指向仅由分隔符组成的字符串，返回值为空字符串（第一个字符为 '\0'），且 *str 被更新为 NULL
- 一般情况：返回值为指向当前 token 起始位置的指针，且 token 的首个分隔符位置被 '\0' 替换
- *str 被更新为指向下一个 token 的起始位置，或为 NULL（若无更多 token）

## 不变量
- *str 指针单调向前移动
- 原始字符串中被 '\0' 替换的分隔符位置不会导致越界

## 算法
1. 从 *str 获取当前 token 起始位置 s
2. 使用 strcspn(s, sep) 找到第一个分隔符位置
3. 若该位置为 '\0'（字符串末尾），将 *str 设为 NULL
4. 否则用 '\0' 覆盖分隔符，将 *str 设为分隔符后下一个位置
5. 返回 s

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strcspn(const char *, const char *);  // 依赖1: 计算不在 sep 集合中的前缀长度，定义于 <string.h>

Predefined Macros:
  _GNU_SOURCE                                    // 依赖2: 启用 GNU 扩展函数声明

[GUARANTEE]
Exported Interface:
  char *strsep(char **str, const char *sep);  // 本模块保证对外提供的接口签名
