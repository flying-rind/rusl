# strnlen

## 函数签名
```c
size_t strnlen(const char *s, size_t n);
```

## 意图
计算字符串 s 的长度，但最多搜索 n 个字符。若在 n 个字符内未找到 '\0'，则返回 n。

## 前置条件
- `s != NULL`
- 当 `n > 0` 时，s 指向的内存区域至少可读取 min(n, strlen(s)+1) 个字节

## 后置条件
- 若 s 在 n 个字符内包含 '\0'，返回值为 strlen(s)
- 若 s 的前 n 个字符均非 '\0'，返回值为 n
- s 指向的内存内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 memchr(s, 0, n) 在 n 字节范围内搜索 '\0'。若找到则返回偏移量，否则返回 n。

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memchr(const void *s, int c, size_t n);  // 依赖1: 在内存块中搜索指定字符

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  size_t strnlen(const char *s, size_t n);
