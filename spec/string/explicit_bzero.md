# explicit_bzero

## 函数签名
```c
void explicit_bzero(void *d, size_t n);
```

## 意图
将 d 指向内存区域的前 n 个字节全部置零，并通过内存屏障阻止编译器优化掉该清零操作。用于安全擦除敏感数据（如密码、密钥）。

## 前置条件
- `d != NULL`
- 当 `n > 0` 时，`d` 指向的内存区域至少可写入 n 个字节

## 后置条件
- 对于所有 i，0 <= i < n，d[i] == 0
- 编译器不得将清零操作优化移除（通过 `__asm__ __volatile__` 内存屏障保证）

## 不变量
- 无全局或静态状态被修改

## 算法
1. 调用 memset(d, 0, n) 执行实际清零
2. 使用 `__asm__ __volatile__` 内联汇编作为内存屏障，传递 d 指针作为读操作数，声明 "memory" clobber，确保编译器不会将清零优化掉

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memset(void *d, int c, size_t n);  // 依赖1: 执行内存置零
  __asm__ __volatile__("" : : "r"(d) : "memory");  // 依赖2: GCC 内联汇编内存屏障，阻止编译器优化移除 memset

Predefined Macros:
  _BSD_SOURCE  // 依赖3: 启用 BSD 兼容函数声明（explicit_bzero 为 BSD 扩展）

[GUARANTEE]
Exported Interface:
  void explicit_bzero(void *d, size_t n);  // 安全擦除内存（不可被编译器优化移除）
