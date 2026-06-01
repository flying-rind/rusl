# wcscasecmp_l

## 函数签名
```c
int wcscasecmp_l(const wchar_t *l, const wchar_t *r, locale_t locale);
```

## 意图
在指定 locale 下忽略大小写比较两个宽字符串。

## 前置条件
- `l != NULL`
- `r != NULL`
- l 和 r 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 与 wcscasecmp(l, r) 返回相同的值
- l 和 r 指向的字符串内容不变

## 不变量
- locale 参数被忽略（musl 当前实现不支持 per-locale 宽字符大小写比较）

## 算法
直接委托给 wcscasecmp(l, r)，忽略 locale 参数。

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  int wcscasecmp(const wchar_t *l, const wchar_t *r);  // 依赖1: 忽略大小写宽字符串比较

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  int wcscasecmp_l(const wchar_t *l, const wchar_t *r, locale_t locale);  // 本模块导出的函数签名
```
