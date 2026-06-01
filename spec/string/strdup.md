# strdup

## 函数签名
```c
char *strdup(const char *s);
```

## 意图
创建字符串 s 的副本，通过 malloc 分配足够的内存并复制内容。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == NULL 当且仅当 malloc 失败
- 若返回值 != NULL：返回值指向的字符串内容与 s 完全相同，包括终止 null
- 返回的副本分配于堆上，调用者负责调用 free() 释放

## 不变量
- 无全局或静态状态被修改

## 算法
1. 计算 s 长度 l = strlen(s)
2. 分配 l+1 字节内存
3. 若分配成功，使用 memcpy 复制 s 的 l+1 字节（包括 '\0'）
4. 若分配失败，返回 NULL

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 长度类型
  strlen(const char *s);  // 依赖2: 计算源字符串长度
  malloc(size_t size);  // 依赖3: 分配堆内存
  memcpy(void *dest, const void *src, size_t n);  // 依赖4: 复制字符串内容到新分配的内存

Predefined Macros:
  NULL;  // 依赖1: 空指针常量

[GUARANTEE]
Exported Interface:
  char *strdup(const char *s);  // 复制字符串到新分配的堆内存
