# bcmp

## 函数签名
```c
int bcmp(const void *s1, const void *s2, size_t n);
```

## 意图
比较内存区域 s1 和 s2 的前 n 个字节是否相等。

## 前置条件
- `s1 != NULL`
- `s2 != NULL`
- 当 `n > 0` 时，`s1` 和 `s2` 指向的内存区域各自至少可读取 n 个字节

## 后置条件
- 返回值 == 0 当且仅当 s1 和 s2 的前 n 个字节完全相同
- 返回值 != 0 当且仅当存在某个字节位置 i (0 <= i < n) 使得 s1[i] != s2[i]
- s1 和 s2 指向的内存区域内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 memcmp(s1, s2, n)，语义等价。

/* Rely */
[RELY]
Predefined Structures/Functions:
  int memcmp(const void *, const void *, size_t);  // 依赖1: 内存比较原语，定义于 <string.h>
  size_t;                                          // 依赖2: 无符号整数类型，定义于 <stddef.h>

Predefined Macros:
  _BSD_SOURCE                                      // 依赖3: 特性测试宏，暴露 BSD 扩展函数声明

[GUARANTEE]
Exported Interface:
  int bcmp(const void *s1, const void *s2, size_t n);  // 本模块保证对外提供的接口签名
