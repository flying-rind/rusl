# memmove

## 函数签名
```c
void *memmove(void *dest, const void *src, size_t n);
```

## 意图
将 src 指向内存区域的前 n 个字节复制到 dest 指向的内存区域。与 memcpy 不同，memmove 正确处理源和目标区域重叠的情况。

## 前置条件
- `dest != NULL`
- `src != NULL`
- 当 `n > 0` 时，`dest` 至少可写入 n 个字节
- 当 `n > 0` 时，`src` 至少可读取 n 个字节

## 后置条件
- dest[0..n-1] == src[0..n-1]（如同先将 src 复制到临时缓冲区再从临时缓冲区复制到 dest，即使区域重叠也能正确复制）
- 返回值为 dest

## 不变量
- 若 d==s，直接返回（无需操作）
- 若 dest 和 src 不重叠（`s-d-n <= -2*n`），可安全使用 memcpy 优化
- 正向复制（d<s）和反向复制（d>s）两条路径各自保证不会覆盖尚未读取的源数据

## 算法
1. 若 d==s，直接返回
2. 若满足不重叠条件（间距绝对值判断），委托给 memcpy
3. 若 d < s（dest 在低地址，src 在高地址）：
   - 若对齐同，逐字节到达字对齐边界后以 WT 为单位正向复制
   - 剩余逐字节复制
4. 若 d > s（dest 在高地址，src 在低地址）：
   - 若对齐相同，从末尾开始以 WT 为单位反向复制
   - 剩余逐字节反向复制

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memcpy(void *dest, const void *src, size_t n);  // 依赖1: 当 dest 和 src 不重叠时委托给 memcpy 完成复制
  size_t;             // 依赖2: 标准无符号整数类型，用于长度参数 n 和 WT 别名
  uintptr_t;          // 依赖3: 标准整数类型，用于指针算术运算和地址对齐判断
  void *;             // 依赖4: 通用指针类型，函数参数和返回值
  NULL;               // 依赖5: 空指针常量

Predefined Macros:
  __GNUC__;           // 编译宏: 启用 GCC 特有的 __attribute__((__may_alias__)) 字复制优化路径
  __attribute__((__may_alias__));  // 编译属性: 允许 WT 指针别名任意类型，绕过严格别名规则

[GUARANTEE]
Exported Interface:
  void *memmove(void *dest, const void *src, size_t n);  // 在 dest 和 src 可能重叠的情况下复制 n 字节
