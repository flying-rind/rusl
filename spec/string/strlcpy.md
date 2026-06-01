# strlcpy

## 函数签名
```c
size_t strlcpy(char *d, const char *s, size_t n);
```

## 意图
将字符串 s 复制到大小为 n 的缓冲区 d 中，始终保证 null 终止（只要 n > 0）。返回所需的总大小（即 strlen(s)）。

## 前置条件
- `d != NULL` 或 `n == 0`
- `s != NULL`
- 当 `n > 0` 时，`d` 指向的缓冲区至少可写入 n 个字节
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 n > 0，d[0..min(strlen(s), n-1)] == s[0..min(strlen(s), n-1)]，且 d[min(strlen(s), n-1)] == '\0'
- 返回值为 strlen(s)（始终返回源字符串长度，而非写入的字节数）

## 不变量
- 最多写入 n 个字节到 d
- 字对齐优化路径中 n 递减确保不超出边界

## 算法
1. 若 n==0，跳转到 finish 返回 strlen(s)
2. 使用字对齐优化（类似 stpncpy 的对齐路径）尽可能快地复制
3. 逐字节复制直到 n 耗尽或遇到 '\0'
4. 在最后一个写入位置后添加 '\0'
5. 返回 strlen(s)

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strlen(const char *s);  // 依赖1: 计算源字符串长度(不含null)用于返回值

Predefined Macros:
  _BSD_SOURCE       // 特性测试宏，启用BSD扩展接口

[GUARANTEE]
Exported Interface:
  size_t strlcpy(char *d, const char *s, size_t n);
