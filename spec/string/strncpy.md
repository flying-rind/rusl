# strncpy

## 函数签名
```c
char *strncpy(char *restrict d, const char *restrict s, size_t n);
```

## 意图
将 s 指向的字符串中最多 n 个字符复制到 d。若 s 长度小于 n，剩余位置用 '\0' 填充。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- `d` 指向的缓冲区至少可写入 n 个字节
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- d[0..min(strlen(s), n)-1] == s[0..min(strlen(s), n)-1]
- 若 strlen(s) < n，d[strlen(s)..n-1] 全部为 '\0'
- 返回值为 d

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 __stpncpy(d, s, n)，忽略其返回值，返回 d。

/* Rely */
[RELY]
Predefined Structures/Functions:
  char *__stpncpy(char *restrict d, const char *restrict s, size_t n);  // 依赖1: 实际执行拷贝的底层函数

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strncpy(char *restrict d, const char *restrict s, size_t n);
