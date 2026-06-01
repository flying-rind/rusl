# memccpy

## 函数签名
```c
void *memccpy(void *restrict dest, const void *restrict src, int c, size_t n);
```

## 意图
从 src 复制字节到 dest，直到发生以下任一条件：(1) 已复制 n 个字节，(2) 遇到字符 c。d==若遇到 c，则将其复制后停止，返回 dest 中 c 之后的下一个字节位置。

## 前置条件
- `dest != NULL`
- `src != NULL`
- `dest` 和 `src` 不重叠（restrict 约束）
- `dest` 至少可写入 n 个字节
- `src` 至少可读取 n 个字节

## 后置条件
- 若在 n 字节内遇到字符 c：dest[0..k-1] == src[0..k-1]（其中 src[k-1]==c, k<=n），返回值为 dest+k
- 若在 n 字节内未遇到字符 c：dest[0..n-1] == src[0..n-1]，返回值为 0

## 不变量
- 复制过程中 dest 偏移量 <= n
- 循环中每次迭代处理最多 sizeof(size_t) 字节（字对齐优化路径）

## 算法
1. 将 c 转换为 unsigned char，并计算 k = ONES * c 用于字宽度的零检测
2. 若 src 和 dest 对齐方式相同，先逐字节复制直到对齐边界或遇到 c
3. 使用 HASZERO 技巧在字级别快速检查是否包含字符 c：检查 (*ws ^ k) 是否包含零字节
4. 剩余字节逐字节复制处理
5. 若在 n 字节内未找到 c，返回 0

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 无符号整数类型，用于内存大小、对齐掩码和字常量计算
  uintptr_t;  // 依赖2: 无符号整数类型，足以容纳指针值，用于地址对齐检测
  unsigned char;  // 依赖3: 单字节无符号类型，用于逐字节遍历和字符比较
  __attribute__((__may_alias__));  // 依赖4: GCC 类型属性，允许 word 指针对任意类型进行别名访问

Predefined Macros:
  __GNUC__  // 依赖5: GCC 编译检测，启用字级别并行复制优化路径
  ALIGN  // 依赖6: 对齐掩码 (sizeof(size_t)-1)
  ONES  // 依赖7: 全 1 字节掩码 ((size_t)-1/UCHAR_MAX)
  HIGHS  // 依赖8: 高位掩码 (ONES * (UCHAR_MAX/2+1))
  HASZERO(x)  // 依赖9: 检测字中是否包含零字节的宏（用于字级别字符匹配检测）

[GUARANTEE]
Exported Interface:
  void *memccpy(void *restrict dest, const void *restrict src, int c, size_t n);  // 从 src 复制到 dest，直到遇到字符 c 或已复制 n 字节
