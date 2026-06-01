# strndup

## 函数签名
```c
char *strndup(const char *s, size_t n);
```

## 意图
创建字符串 s 的副本，但最多复制 n 个字符。通过 malloc 分配内存。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == NULL 当且仅当 malloc 失败
- 若返回值 != NULL：返回的字符串包含 s 的前 l = min(strlen(s), n) 个字符，且以 '\0' 终止
- 返回的副本分配于堆上，调用者负责调用 free() 释放

## 不变量
- 无全局或静态状态被修改

## 算法
1. 计算长度 l = strnlen(s, n)
2. 分配 l+1 字节
3. 若成功：memcpy(d, s, l) 复制 l 字节，d[l] = 0
4. 若失败：返回 NULL

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strnlen(const char *s, size_t n);  // 依赖1: 计算有限长度(最多n个字符)
  void *malloc(size_t size);                // 依赖2: 动态分配堆内存
  void *memcpy(void *d, const void *s, size_t n);  // 依赖3: 复制内存块到新分配空间

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strndup(const char *s, size_t n);
