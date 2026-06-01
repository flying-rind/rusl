# wcscspn

## 函数签名
```c
size_t wcscspn(const wchar_t *s, const wchar_t *c);
```

## 意图
计算 s 的起始段长度，该段中不包含宽字符串 c 中的任何宽字符。

## 前置条件
- `s != NULL`
- `c != NULL`
- s 和 c 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 返回值 == min{ i >= 0 | s[i] == L'\0' 或 s[i] 属于 c }
- s 和 c 指向的字符串内容不变

## 不变量
- s 指针单调递增

## 算法
1. 若 c 为空串，返回 wcslen(s)
2. 若 c 仅一个字符，使用 wcschr 快速查找
3. 一般情况：对 s 中每个字符调用 wcschr(c, *s) 检查是否属于 c，直到匹配或到达结尾

注意：该算法为 O(|s| * |c|) 的朴素搜索（因宽字符集太大无法使用位图）。

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcslen(const wchar_t *s);             // 依赖1: 宽字符串长度计算（用于空 c 或快速路径）
  wchar_t *wcschr(const wchar_t *s, wchar_t c);  // 依赖2: 宽字符查找（用于快速路径和一般情况）

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  size_t wcscspn(const wchar_t *s, const wchar_t *c);  // 本模块导出的函数签名
```
