# wcsstr

## 函数签名
```c
wchar_t *wcsstr(const wchar_t *restrict h, const wchar_t *restrict n);
```

## 意图
在宽字符串 h（haystack）中查找子串 n（needle）第一次出现的位置。

## 前置条件
- `h != NULL`
- `n != NULL`
- h 和 n 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 若 n[0] == L'\0'，返回值为 (wchar_t*)h
- 若 h[0] == L'\0' 且 n 非空，返回值为 NULL
- 若在 h 中找到 n，返回值为 h 中第一个匹配位置的指针
- 若未找到，返回值为 NULL
- h 和 n 指向的字符串内容不变

## 不变量
- 搜索使用递增的 h 指针，从不回溯超过已检查范围

## 算法
1. 若 n 为空串，返回 h
2. 若 h 为空串，返回 NULL
3. 使用 wcschr 跳跃到 n 首字符在 h 中首次出现位置
4. 若 n 长度为 1 或 2，由 wcschr 已完成
5. 对于更长的 needle：使用 twoway_wcsstr

### twoway_wcsstr 算法细节
- 计算 needle 长度，同时建立 h 的边界
- 使用最大后缀算法计算 needle 的自相似性（两次：左向和右向）
- 检测 needle 是否为周期串，利用周期性加速
- 使用 wmemchr 逐步扩展 haystack 结束边界 z
- 双向比较：先比较右半部分（最大后缀之后），再比较左半部分（最大后缀之前）

/* Rely */
[RELY]
Predefined Structures/Functions:
  wchar_t *wcschr(const wchar_t *s, wchar_t c);  // 依赖1: 在h中查找needle首字符
  wchar_t *wmemchr(const wchar_t *s, wchar_t c, size_t n);  // 依赖2: 在h中查找空字符以扩展边界
  int wmemcmp(const wchar_t *l, const wchar_t *r, size_t n);  // 依赖3: 判断needle是否为周期串

Predefined Macros:
  MAX(a,b)  // 内部宏: 取较大值
  MIN(a,b)  // 内部宏: 取较小值

[GUARANTEE]
Exported Interface:
  wchar_t *wcsstr(const wchar_t *restrict h, const wchar_t *restrict n);
