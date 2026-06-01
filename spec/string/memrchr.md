# memrchr

## 函数签名
```c
void *memrchr(const void *m, int c, size_t n);
```

## 意图
在 m 指向的内存区域的前 n 个字节中从后向前查找字符 c 最后一次出现的位置。

## 前置条件
- `m != NULL`
- 当 `n > 0` 时，`m` 指向的内存区域至少可读取 n 个字节

## 后置条件
- 若存在 i，0 <= i < n，使得 ((unsigned char*)m)[i] == (unsigned char)c，且对于所有 j > i 且 j < n，m[j] != c，则返回值为 (void*)(m + i)
- 若不存在，返回值为 NULL
- m 指向的内存内容不变

## 不变量
- 搜索从索引 n-1 向 0 递减，每次迭代 n 递减 1，循环终止于 n==0
- 搜索位置始终在 [0, n) 区间内

## 算法
从末尾向开头遍历 m[n-1] 到 m[0]，返回第一个匹配位置的指针。通过 weak_alias 暴露为 memrchr。

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;             // 依赖1: 标准无符号整数类型，用于长度参数 n
  void *;             // 依赖2: 通用指针类型，函数参数和返回值
  unsigned char;      // 依赖3: 用于无符号字节比较，确保 c 在 unsigned 域内比较
  NULL;               // 依赖4: 空指针常量，未找到时返回值

Predefined Macros:
  weak_alias(__memrchr, memrchr);  // 内部宏: 将 __memrchr 导出为弱符号 memrchr

[GUARANTEE]
Exported Interface:
  void *__memrchr(const void *m, int c, size_t n);  // 内部实现，从后向前查找字符
  void *memrchr(const void *m, int c, size_t n);    // 弱别名，公开接口
