# stpcpy

## 函数签名
```c
char *stpcpy(char *restrict d, const char *restrict s);
```

## 意图
将 s 指向的字符串（包括终止 null）复制到 d 指向的缓冲区，返回 d 中终止 null 字符的位置（而非 d 起始地址）。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 指向的缓冲区至少可写入 strlen(s) + 1 个字节
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- d[0..len] == s[0..len]（其中 len == strlen(s)），包括终止 null
- 返回值为 &d[len]

## 不变量
- d 和 s 指针增量相等
- 字对齐优化路径中每次处理 sizeof(size_t) 个字节，通过 HASZERO 检测 null 终止符

## 算法
1. 若 d 和 s 对齐方式相同：
   - 逐字节复制直到 s 到达字对齐边界
   - 使用 HASZERO 宏在字级别快速检测 null 终止符
   - 以字为单位批量复制
2. 剩余部分逐字节复制
3. 返回指向 d 中终止 null 的指针

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;             // 依赖1: 标准无符号整数类型，用于字大小 ALIGN、HASZERO 运算
  uintptr_t;          // 依赖2: 标准整数类型，用于地址对齐检测
  void *;             // 依赖3: 通用指针类型，用于类型双关
  UCHAR_MAX;          // 依赖4: <limits.h> 常量，用于 ONES/HIGHS 宏计算
  restrict;           // 依赖5: 类型限定关键字，标明 d 和 s 不重叠
  NULL;               // 依赖6: 空指针常量

Predefined Macros:
  __GNUC__;           // 编译宏: 启用 GCC 字对齐批量复制优化路径
  __attribute__((__may_alias__));  // 编译属性: 允许 word 指针别名任意类型
  weak_alias(__stpcpy, stpcpy);    // 内部宏: 将 __stpcpy 导出为弱符号 stpcpy
  ALIGN;              // 局部宏: sizeof(size_t)，字对齐边界
  ONES;               // 局部宏: ((size_t)-1/UCHAR_MAX)，全 1 字节掩码
  HIGHS;              // 局部宏: ONES * (UCHAR_MAX/2+1)，高位掩码
  HASZERO(x);         // 局部宏: 检测 size_t 字中是否有零字节

[GUARANTEE]
Exported Interface:
  char *__stpcpy(char *restrict d, const char *restrict s);  // 内部实现，复制字符串返回末尾指针
  char *stpcpy(char *restrict d, const char *restrict s);    // 弱别名，公开接口
