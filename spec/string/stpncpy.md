# stpncpy

## 函数签名
```c
char *stpncpy(char *restrict d, const char *restrict s, size_t n);
```

## 意图
将 s 指向的字符串中最多 n 个字符复制到 d。若 s 长度小于 n，剩余部分用 '\0' 填充。返回 d + min(strlen(s), n) 即写入的最后一个非 null 字符之后的位置。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 指向的缓冲区至少可写入 n 个字节
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- d[0..k-1] == s[0..k-1]（其中 k == min(strlen(s), n)）
- 若 strlen(s) < n，则 d[k..n-1] == 0
- 返回值为 d + k

## 不变量
- 剩余数量 n 持续递减，确保不超出边界
- 字对齐路径中每次处理 sizeof(size_t) 字节，通过 HASZERO 检测 null

## 算法
1. 若 d 和 s 对齐方式相同：
   - 逐字节复制直到对齐或 n 耗尽或遇到 null
   - 使用 HASZERO 在字级别批量复制
2. 逐字节复制剩余部分
3. 用 memset 将剩余位置填充为 0

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memset(void *dest, int c, size_t n);  // 依赖1: 将剩余位置填充为 0
  size_t;             // 依赖2: 标准无符号整数类型，用于长度参数 n、字对齐运算
  uintptr_t;          // 依赖3: 标准整数类型，用于地址对齐检测
  void *;             // 依赖4: 通用指针类型，用于类型双关
  UCHAR_MAX;          // 依赖5: <limits.h> 常量，用于 ONES/HIGHS 宏计算
  restrict;           // 依赖6: 类型限定关键字，标明 d 和 s 不重叠
  NULL;               // 依赖7: 空指针常量

Predefined Macros:
  __GNUC__;           // 编译宏: 启用 GCC 字对齐批量复制优化路径
  __attribute__((__may_alias__));  // 编译属性: 允许 word 指针别名任意类型
  weak_alias(__stpncpy, stpncpy);  // 内部宏: 将 __stpncpy 导出为弱符号 stpncpy
  ALIGN;              // 局部宏: sizeof(size_t)-1，对齐掩码
  ONES;               // 局部宏: ((size_t)-1/UCHAR_MAX)，全 1 字节掩码
  HIGHS;              // 局部宏: ONES * (UCHAR_MAX/2+1)，高位掩码
  HASZERO(x);         // 局部宏: 检测 size_t 字中是否有零字节

[GUARANTEE]
Exported Interface:
  char *__stpncpy(char *restrict d, const char *restrict s, size_t n);  // 内部实现，复制最多 n 字符，返回末尾指针
  char *stpncpy(char *restrict d, const char *restrict s, size_t n);    // 弱别名，公开接口
