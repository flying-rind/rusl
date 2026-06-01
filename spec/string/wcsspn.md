# wcsspn

## 函数签名
```c
size_t wcsspn(const wchar_t *s, const wchar_t *c);
```

## 意图
计算 s 的起始段长度，该段中所有宽字符都属于集合 c。

## 前置条件
- `s != NULL`
- `c != NULL`
- s 和 c 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 返回值 == min{ i >= 0 | s[i] == L'\0' 或 s[i] 不属于 c }
- s 和 c 指向的字符串内容不变

## 不变量
- s 指针单调递增直到不属于 c 的字符或 L'\0'

## 算法
对 s 的每个字符调用 wcschr(c, *s) 检查其是否在 c 中。注意：该算法为 O(|s| * |c|) 的朴素搜索。

/* Rely */
[RELY]
Predefined Structures/Functions:
  wchar_t *wcschr(const wchar_t *s, wchar_t c);  // 依赖1: 在c集合中查找字符

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  size_t wcsspn(const wchar_t *s, const wchar_t *c);
