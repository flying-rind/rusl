# strncat

## 函数签名
```c
char *strncat(char *restrict d, const char *restrict s, size_t n);
```

## 意图
将字符串 s 中最多 n 个字符追加到 d 末尾。若 s 长度小于 n，则仅追加到 s 的 null 终止符为止。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- d 指向以 null 结尾的有效 C 字符串
- s 指向以 null 结尾的有效 C 字符串
- d 指向的缓冲区至少可容纳 strlen(d) + min(n, strlen(s)) + 1 个字节

## 后置条件
- d 末尾追加了 s 的前 min(n, strlen(s)) 个字符，然后追加 '\0'
- d 原有内容不变（除追加的位置外）
- 返回值为 d

## 不变量
- 最多向 d 末尾写入 min(n, strlen(s)) + 1 个字节（数据 + null）
- 指针 d 先移到终止 null 位置

## 算法
1. 将 d 移动到 strlen(d) 的位置（即 '\0' 处）
2. 循环：最多 n 次，每次复制 s 中的一个字符直到 '\0'
3. 追加终止 '\0'
4. 返回原 d 指针（保存的 a）

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t strlen(const char *s);  // 依赖1: 找到目标字符串d末尾的null位置

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strncat(char *restrict d, const char *restrict s, size_t n);
