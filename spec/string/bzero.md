# bzero

## 函数签名
```c
void bzero(void *s, size_t n);
```

## 意图
将 s 指向内存区域的前 n 个字节全部置零。

## 前置条件
- `s != NULL`
- 当 `n > 0` 时，`s` 指向的内存区域至少可写入 n 个字节

## 后置条件
- 对于所有 i，0 <= i < n，s[i] == 0

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 memset(s, 0, n)。

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memset(void *d, int c, size_t n);  // 依赖1: 执行内存置零操作

Predefined Macros:
  _BSD_SOURCE  // 依赖2: 启用 BSD 兼容函数声明（bzero 为 BSD 扩展）

[GUARANTEE]
Exported Interface:
  void bzero(void *s, size_t n);  // 将 s 指向内存的前 n 字节置零
