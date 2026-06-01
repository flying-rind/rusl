# strlcat

## 函数签名
```c
size_t strlcat(char *d, const char *s, size_t n);
```

## 意图
将字符串 s 追加到大小为 n 的缓冲区 d 中已有字符串之后，始终保证 null 终止（只要 n > 0）。返回所需的缓冲区大小。

## 前置条件
- `d != NULL` 或 `n == 0`
- `s != NULL`
- d 指向以 null 结尾的有效 C 字符串或至少 n 字节的可写内存
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 n > 0，d[n-1] == '\0'
- 返回值 == strlen(d 原内容) + strlen(s)（始终返回"需要"的总大小，而非实际写入的大小）
- 若返回值 < n，则 d 中包含完整的拼接结果

## 不变量
- 不会向 d 写入超过 n 个字节的数据
- n==0 时跳过所有写入操作

## 算法
1. 使用 strnlen 计算 d 在大小为 n 的缓冲区中的长度 l
2. 若 l == n（d 已填满缓冲区，无空间追加）：返回 l + strlen(s)
3. 否则：返回 l + strlcpy(d+l, s, n-l)

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 长度和返回值类型
  strnlen(const char *s, size_t maxlen);  // 依赖2: 安全地测量 d 在 n 字节缓冲区中的长度
  strlen(const char *s);  // 依赖3: d 已满时计算 s 长度
  strlcpy(char *dest, const char *src, size_t n);  // 依赖4: 将 s 复制到 d 的剩余空间

Predefined Macros:
  _BSD_SOURCE;  // 依赖1: 特性测试宏，启用 BSD 扩展接口

[GUARANTEE]
Exported Interface:
  size_t strlcat(char *d, const char *s, size_t n);  // 安全字符串拼接，限制目标缓冲区大小
