# wmemmove

## 函数签名
```c
wchar_t *wmemmove(wchar_t *d, const wchar_t *s, size_t n);
```

## 意图
将 s 指向的宽字符数组中的前 n 个元素复制到 d，正确处理源和目标区域重叠的情况。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 至少可写入 n 个 wchar_t
- `s` 至少可读取 n 个 wchar_t

## 后置条件
- d[0..n-1] == s[0..n-1]（如同先复制到临时缓冲区，即使重叠也能正确复制）
- 返回值为 d

## 不变量
- 若 d == s，直接返回
- 若重叠且 d > s，使用反向复制（从末尾开始）
- 若重叠且 d < s 或无重叠，使用正向复制

## 算法
1. 若 d == s，直接返回
2. 通过计算指针差判断是否重叠：`(uintptr_t)d - (uintptr_t)s < n * sizeof(*d)`
3. 若重叠（d > s），采用反向复制：while(n--) d[n] = s[n]
4. 否则采用正向复制：while(n--) *d++ = *s++

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wmemmove(wchar_t *d, const wchar_t *s, size_t n);
