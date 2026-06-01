# memcpy

## 函数签名
```c
void *memcpy(void *restrict dest, const void *restrict src, size_t n);
```

## 意图
将 src 指向内存区域的前 n 个字节复制到 dest 指向的内存区域。调用者保证 dest 和 src 不重叠。

## 前置条件
- `dest != NULL`
- `src != NULL`
- `dest` 和 `src` 不重叠（restrict 约束；若重叠，行为未定义）
- `dest` 至少可写入 n 个字节
- `src` 至少可读取 n 个字节

## 后置条件
- dest[0..n-1] == src[0..n-1]
- 返回值为 dest

## 不变量
- 循环处理中 s 和 d 指针的变化量与 n 的变化量保持一致
- 每次循环迭代处理固定数量的字节（1/2/4/8/16），保证不会超出边界

## 算法
1. 若 src 未 4 字节对齐，逐字节复制直到 src 对齐
2. 若 dest 已 4 字节对齐：以 16 字节块为单位使用 uint32_t 复制（按 4 个 4 字节操作），然后依次处理剩余 8/4/2/1 字节
3. 若 dest 未对齐（偏移 1/2/3）：使用位移拼接技术（以本机字节序使用 LS/RS 宏），以 16 字节块处理对齐偏移，然后处理剩余字节
4. 非 GNU C 编译时回退到逐字节循环

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 无符号整数类型，用于内存大小和位移
  uint32_t;  // 依赖2: 32 位无符号整数类型，用于字级别对齐复制和位移拼接
  uintptr_t;  // 依赖3: 无符号整数类型，用于地址对齐检测
  unsigned char;  // 依赖4: 单字节无符号类型，用于逐字节复制
  __attribute__((__may_alias__));  // 依赖5: GCC 类型属性，允许 u32 指针对任意类型进行别名访问

Predefined Macros:
  __GNUC__  // 依赖6: GCC 编译检测，启用字级别优化路径
  __BYTE_ORDER  // 依赖7: 系统字节序定义，由 <endian.h> 提供
  __LITTLE_ENDIAN  // 依赖8: 小端字节序常量，用于条件编译决定 LS/RS 方向
  LS  // 依赖9: 位移宏（小端为 >>，大端为 <<），用于未对齐时的字节序调整
  RS  // 依赖10: 位移宏（小端为 <<，大端为 >>），用于未对齐时的字节序调整

[GUARANTEE]
Exported Interface:
  void *memcpy(void *restrict dest, const void *restrict src, size_t n);  // 从 src 复制 n 字节到 dest（不重叠）
