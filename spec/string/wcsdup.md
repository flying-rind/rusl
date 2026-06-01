# wcsdup

## 函数签名
```c
wchar_t *wcsdup(const wchar_t *s);
```

## 意图
创建宽字符串 s 的堆副本。

## 前置条件
- `s != NULL`
- s 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 返回值 == NULL 当且仅当 malloc 失败
- 若返回值 != NULL：*返回值 == *s（字符串内容相等），包括终止 L'\0'
- 返回的副本分配于堆上，调用者负责调用 free() 释放

## 不变量
- 无全局或静态状态被修改

## 算法
1. 计算长度 l = wcslen(s)
2. 分配 (l+1) * sizeof(wchar_t) 字节
3. 若成功，使用 wmemcpy(d, s, l+1) 复制包括 L'\0' 的内容
4. 若失败，返回 NULL

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcslen(const wchar_t *s);  // 依赖1: 计算宽字符串长度
  void *malloc(size_t size);  // 依赖2: 堆内存分配
  wchar_t *wmemcpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);  // 依赖3: 宽字符内存复制

Predefined Macros:
  NULL  // 空指针常量

[GUARANTEE]
Exported Interface:
  wchar_t *wcsdup(const wchar_t *s);  // 创建宽字符串的堆副本
