# wmemset

## 函数签名
```c
wchar_t *wmemset(wchar_t *d, wchar_t c, size_t n);
```

## 意图
将 d 指向的宽字符数组的前 n 个元素全部设置为宽字符 c。

## 前置条件
- `d != NULL`
- 当 `n > 0` 时，`d` 指向的内存区域至少可写入 n 个 wchar_t

## 后置条件
- 对于所有 i，0 <= i < n，d[i] == c
- 返回值为 d

## 不变量
- n 递减确保恰好设置 n 个元素

## 算法
简单的 while 循环逐元素赋值：while(n--) *d++ = c。

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wmemset(wchar_t *d, wchar_t c, size_t n);
