# strcpy

## 函数签名
```c
char *strcpy(char *restrict dest, const char *restrict src);
```

## 意图
将 src 指向的字符串（包括终止 null）复制到 dest 指向的缓冲区。

## 前置条件
- `dest != NULL`
- `src != NULL`
- `dest` 和 `src` 不重叠（restrict 约束）
- `dest` 指向的缓冲区至少可写入 strlen(src) + 1 个字节
- src 指向以 null 结尾的有效 C 字符串

## 后置条件
- dest[0..len] == src[0..len]（其中 len == strlen(src)），包括终止 null
- 返回值为 dest

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 __stpcpy(dest, src)，忽略其返回值，返回 dest。

/* Rely */
[RELY]
Predefined Structures/Functions:
  char *restrict;  // 依赖1: dest 和 src 指针的 restrict 限定
  __stpcpy(char *restrict dest, const char *restrict src);  // 依赖2: 实际字符串复制实现

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strcpy(char *restrict dest, const char *restrict src);  // 将 src 复制到 dest
