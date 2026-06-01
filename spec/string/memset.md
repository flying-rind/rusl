# memset

## 函数签名
```c
void *memset(void *dest, int c, size_t n);
```

## 意图
将 dest 指向内存区域的前 n 个字节全部设置为值 c（转换为 unsigned char）。

## 前置条件
- `dest != NULL`
- 当 `n > 0` 时，`dest` 指向的内存区域至少可写入 n 个字节

## 后置条件
- 对于所有 i，0 <= i < n，dest[i] == (unsigned char)c
- 返回值为 dest

## 不变量
- 填充分阶段进行：先填充头部和尾部（最少分支处理短区域），再对齐到 4 字节边界，最后以 32 字节块高效填充
- 每个条件分支确保后续使用的偏移量均在有效区域内

## 算法
1. 若 n==0，直接返回
2. 头部/尾部 1-4 字节：通过检查 n<=2, n<=6, n<=8 的条件逐步填充，确保覆盖所有字节
3. 对齐：将 s 指针提升到 4 字节对齐边界，并截断 n 为 4 的倍数
4. 构造 c32 = 0x01010101 * c，将字节复制到 32 位
5. 填充前后各 28 字节（使用 u32 存储），然后对齐到 8 字节边界
6. 构造 c64 = c32 | (c32 << 32)
7. 以 32 字节为单位循环使用 u64 填充
8. 纯 C 回退：逐字节循环

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;             // 依赖1: 标准无符号整数类型，用于长度参数 n、对齐偏移 k
  uintptr_t;          // 依赖2: 标准整数类型，用于获取地址对齐信息
  uint32_t;           // 依赖3: 标准 32 位无符号整数类型，用于 4 字节批量填充
  uint64_t;           // 依赖4: 标准 64 位无符号整数类型，用于 8 字节批量填充
  unsigned char;      // 依赖5: 用于逐字节操作的基础类型
  void *;             // 依赖6: 通用指针类型，函数参数和返回值
  NULL;               // 依赖7: 空指针常量

Predefined Macros:
  __GNUC__;           // 编译宏: 启用 GCC 特有的 __attribute__((__may_alias__)) 和 32/64 位批量填充优化路径
  __attribute__((__may_alias__));  // 编译属性: 允许 u32/u64 指针别名任意类型，绕过严格别名规则

[GUARANTEE]
Exported Interface:
  void *memset(void *dest, int c, size_t n);  // 将 dest 前 n 字节设置为值 c
