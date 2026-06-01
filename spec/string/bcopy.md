# bcopy

## 函数签名
```c
void bcopy(const void *s1, void *s2, size_t n);
```

## 意图
将 s1 指向内存的前 n 个字节复制到 s2 指向的内存区域。与 memcpy 不同，bcopy 允许源和目标区域重叠。

## 前置条件
- `s1 != NULL`
- `s2 != NULL`
- 当 `n > 0` 时，`s1` 指向的内存区域至少可读取 n 个字节
- 当 `n > 0` 时，`s2` 指向的内存区域至少可写入 n 个字节

## 后置条件
- s2[0..n-1] == s1[0..n-1]（按字节复制，支持重叠区域）
- s1 指向的内存区域内容不变（当不重叠时）

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 memmove(s2, s1, n)，利用 memmove 对重叠区域的安全处理能力。

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memmove(void *dest, const void *src, size_t n);  // 依赖1: 执行重叠安全的内存复制

Predefined Macros:
  _BSD_SOURCE  // 依赖2: 启用 BSD 兼容函数声明（bcopy 为 BSD 扩展）

[GUARANTEE]
Exported Interface:
  void bcopy(const void *s1, void *s2, size_t n);  // 从 s1 复制 n 字节到 s2（支持重叠区域）
