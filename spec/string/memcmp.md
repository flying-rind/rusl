# memcmp

## 函数签名
```c
int memcmp(const void *vl, const void *vr, size_t n);
```

## 意图
比较 vl 和 vr 指向的内存区域的前 n 个字节。

## 前置条件
- `vl != NULL`
- `vr != NULL`
- 当 `n > 0` 时，`vl` 和 `vr` 指向的内存区域各自至少可读取 n 个字节

## 后置条件
- 返回值 == 0：所有 n 个字节对应相等，或 n == 0
- 返回值 < 0：在第一个不相等的字节位置 i（0 <= i < n）处，vl[i] < vr[i]
- 返回值 > 0：在第一个不相等的字节位置 i（0 <= i < n）处，vl[i] > vr[i]
- vl 和 vr 指向的内存区域内容不变

## 不变量
- 比较的字节数从未超过 n
- 循环结束时，若所有前 n 字节相等则 l 和 r 恰好指向对应区域的末尾后一个字节

## 算法
逐字节比较，在第一个不相等处返回差值 `*l - *r`。若全部相等且 n == 0，返回 0。

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 无符号整数类型，用于字节计数
  unsigned char;  // 依赖2: 单字节无符号类型，用于逐字节比较和无符号差值计算

Predefined Macros:
  (none)  // 无特性测试宏依赖；memcmp 为 C 标准函数，使用时仅需包含 <string.h>

[GUARANTEE]
Exported Interface:
  int memcmp(const void *vl, const void *vr, size_t n);  // 比较两个内存区域的前 n 字节
