# mempcpy

## 函数签名
```c
void *mempcpy(void *dest, const void *src, size_t n);
```

## 意图
将 src 指向内存区域的前 n 个字节复制到 dest，返回指向 dest 中最后一个写入字节之后的位置（即 dest+n），而非 dest 本身。

## 前置条件
- `dest != NULL`
- `src != NULL`
- `dest` 至少可写入 n 个字节
- `src` 至少可读取 n 个字节
- `dest` 和 `src` 不重叠（委托给 memcpy）

## 后置条件
- dest[0..n-1] == src[0..n-1]
- 返回值为 (char*)dest + n

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 memcpy(dest, src, n)，返回 memcpy 结果 + n 偏移量。

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memcpy(void *dest, const void *src, size_t n);  // 依赖1: 完成实际的字节复制工作
  size_t;             // 依赖2: 标准无符号整数类型，用于长度参数 n
  void *;             // 依赖3: 通用指针类型，函数参数和返回值

Predefined Macros:
  _GNU_SOURCE;        // 特性测试宏: 启用 GNU 扩展函数 mempcpy 的定义

[GUARANTEE]
Exported Interface:
  void *mempcpy(void *dest, const void *src, size_t n);  // 复制 n 字节并返回 dest + n
