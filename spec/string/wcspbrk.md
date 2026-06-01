# wcspbrk

## 函数签名
```c
wchar_t *wcspbrk(const wchar_t *s, const wchar_t *b);
```

## 意图
在宽字符串 s 中查找 b 中任意宽字符第一次出现的位置。

## 前置条件
- `s != NULL`
- `b != NULL`
- s 和 b 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 若存在 i >= 0 使得 s[i] 属于 b（且 s[i] != L'\0'），返回值为 &s[i]
- 若不存在匹配，返回值为 NULL
- s 和 b 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
使用 wcscspn(s, b) 计算首个匹配位置的偏移量，若该位置非 L'\0' 则返回其指针，否则返回 NULL。

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcscspn(const wchar_t *s, const wchar_t *b);  // 依赖1: 计算在b中任意字符首次出现前的跨度

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  wchar_t *wcspbrk(const wchar_t *s, const wchar_t *b);  // 查找b中任意宽字符在s中的首次出现
