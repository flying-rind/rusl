# wcscmp

## 函数签名
```c
int wcscmp(const wchar_t *l, const wchar_t *r);
```

## 意图
比较两个宽字符串 l 和 r。

## 前置条件
- `l != NULL`
- `r != NULL`
- l 和 r 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 返回值 == 0：l 和 r 完全相同
- 返回值 == -1：l 在字典序上小于 r
- 返回值 == 1：l 在字典序上大于 r
- l 和 r 指向的字符串内容不变

## 不变量
- l 和 r 指针增量相等
- 循环到第一个不同字符或同时到达结尾时终止

## 算法
逐字符比较，直到 *l != *r 或 *l == L'\0'。返回值限定为 -1、0 或 1（而非原始差值）。

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)  // 纯内联实现，不调用任何外部函数

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  int wcscmp(const wchar_t *l, const wchar_t *r);  // 本模块导出的函数签名
```
