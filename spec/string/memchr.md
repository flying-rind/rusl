# memchr

## 函数签名
```c
void *memchr(const void *src, int c, size_t n);
```

## 意图
在 src 指向的内存区域的前 n 个字节中查找字符 c 第一次出现的位置。

## 前置条件
- `src != NULL`
- 当 `n > 0` 时，`src` 指向的内存区域至少可读取 n 个字节

## 后置条件
- 若存在 i，0 <= i < n，使得 ((unsigned char*)src)[i] == (unsigned char)c，返回值为 (void*)(src + i)
- 若不存在，返回值为 NULL
- src 指向的内存内容不变

## 不变量
- 搜索位置从不越界，始终满足 当前偏移量 + 剩余字节数 == n
- 在字级别搜索时，HASZERO 保证了零字节检测的正确性

## 算法
1. 将 c 转换为 unsigned char，并计算 k = ONES * c 用于字宽度匹配
2. 逐字节前进直到 src 指针达到字对齐边界
3. 以 sizeof(size_t) 为步长进行字级别搜索，使用 HASZERO(*w ^ k) 快速检测字中是否包含目标字节
4. 找到可能包含目标字节的字后，退回到字节级别精确定位

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 无符号整数类型，用于内存大小、步进和常量计算
  uintptr_t;  // 依赖2: 无符号整数类型，用于地址对齐检测
  unsigned char;  // 依赖3: 单字节无符号类型，用于逐字节搜索和字符比较
  __attribute__((__may_alias__));  // 依赖4: GCC 类型属性，允许 word 指针对任意类型进行别名访问

Predefined Macros:
  __GNUC__  // 依赖5: GCC 编译检测，启用字级别并行搜索优化路径
  SS  // 依赖6: sizeof(size_t)，字大小常量
  ALIGN  // 依赖7: 对齐掩码 (sizeof(size_t)-1)
  ONES  // 依赖8: 全 1 字节掩码 ((size_t)-1/UCHAR_MAX)
  HIGHS  // 依赖9: 高位掩码 (ONES * (UCHAR_MAX/2+1))
  HASZERO(x)  // 依赖10: 检测字中是否包含零字节的宏（用于字级别字符匹配检测）

[GUARANTEE]
Exported Interface:
  void *memchr(const void *src, int c, size_t n);  // 在 src 的前 n 字节中查找字符 c 第一次出现的位置
