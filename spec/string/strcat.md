# strcat

## 函数签名
```c
char *strcat(char *restrict dest, const char *restrict src);
```

## 意图
将 src 字符串的内容追加到 dest 字符串的末尾（覆盖 dest 的终止 null），包括 src 的终止 null。

## 前置条件
- `dest != NULL`
- `src != NULL`
- `dest` 和 `src` 不重叠（restrict 约束）
- dest 指向以 null 结尾的有效 C 字符串
- src 指向以 null 结尾的有效 C 字符串
- dest 指向的缓冲区至少可容纳 strlen(dest) + strlen(src) + 1 个字节

## 后置条件
- dest[dest_len..dest_len+src_len] == src[0..src_len]（其中 dest_len==strlen(dest), src_len==strlen(src)）
- dest[dest_len] 为原 src 的第一个字符
- dest[dest_len+src_len] == '\0'
- 返回值为 dest

## 不变量
- 无全局或静态状态被修改

## 算法
1. 通过 strlen(dest) 找到 dest 末尾的 '\0'
2. 通过 strcpy 将 src 复制到该位置
3. 返回 dest

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: strlen 返回值和参数中的长度类型
  char *restrict;  // 依赖2: dest 和 src 指针的 restrict 限定
  strlen(const char *s);  // 依赖3: 计算 dest 字符串长度
  strcpy(char *restrict dest, const char *restrict src);  // 依赖4: 将 src 复制到 dest 末尾

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strcat(char *restrict dest, const char *restrict src);  // 将 src 追加到 dest 末尾
