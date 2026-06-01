# strlen

## 函数签名
```c
size_t strlen(const char *s);
```

## 意图
计算字符串 s 的长度（不包含终止 null 字符）。

## 前置条件
- `s != NULL`
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == 满足 s[i] == '\0' 的最小 i
- s 指向的字符串内容不变

## 不变量
- 搜索指针从 s 开始，持续递增直到找到 '\0'
- 字级别搜索时 HASZERO 宏保证正确检测 null 字节

## 算法
1. 逐字节前进直到 s 到达字对齐地址（ALIGN == sizeof(size_t)），同时检查 '\0'
2. 使用 HASZERO 宏在字级别快速检测 null 字节
3. 找到包含 null 的字后，回退到字节级别精确定位
4. 返回 s - a（终止 null 指针减去起始指针）

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)  // 纯算法实现，无外部函数调用

Predefined Macros:
  (none)  // 所有辅助宏(ALIGN/ONES/HIGHS/HASZERO)均在本地定义

[GUARANTEE]
Exported Interface:
  size_t strlen(const char *s);
