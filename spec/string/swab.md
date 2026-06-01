# swab

## 函数签名
```c
void swab(const void *restrict _src, void *restrict _dest, ssize_t n);
```

## 意图
将 _src 中的 n 个字节相邻两两交换后复制到 _dest。即 dest[0]=src[1], dest[1]=src[0], dest[2]=src[3], dest[3]=src[2], ... 若 n 为奇数，最后 1 个字节不处理。

## 前置条件
- `_src != NULL`
- `_dest != NULL`
- `_src` 和 `_dest` 不重叠（restrict 约束）
- `_src` 至少可读取 n 个字节
- `_dest` 至少可写入 (n & ~1) 个字节（偶数个字节）

## 后置条件
- 对于所有 i，0 <= i < n/2：dest[2*i] == src[2*i+1] 且 dest[2*i+1] == src[2*i]
- _src 中的剩余字节（若 n 为奇数）不被复制
- _src 指向的内存内容不变

## 不变量
- 循环处理成对字节，n 每次递减 2
- dest 和 src 指针每次递增 2

## 算法
逐对处理：每次迭代交换 src[0]/src[1] 到 dest[1]/dest[0]，n 递减 2。循环终止于 n <= 1。

/* Rely */
[RELY]
Predefined Structures/Functions:
  ssize_t;                                       // 依赖1: 有符号整数类型，定义于 <unistd.h>

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  void swab(const void *restrict _src, void *restrict _dest, ssize_t n);  // 本模块保证对外提供的接口签名
