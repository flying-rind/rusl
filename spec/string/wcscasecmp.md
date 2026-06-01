# wcscasecmp

## 函数签名
```c
int wcscasecmp(const wchar_t *l, const wchar_t *r);
```

## 意图
在忽略大小写的情况下比较两个宽字符串。

## 前置条件
- `l != NULL`
- `r != NULL`
- l 和 r 指向以 L'\0' 结尾的有效宽字符串

## 后置条件
- 返回值 == 0：两个宽字符串忽略大小写后相等
- 返回值 < 0：l 在字典序上小于 r（忽略大小写）
- 返回值 > 0：l 在字典序上大于 r（忽略大小写）
- l 和 r 指向的字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 wcsncasecmp(l, r, -1)，将 size_t(-1) 作为无限制的长度参数。

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  int wcsncasecmp(const wchar_t *l, const wchar_t *r, size_t n);  // 依赖1: 定长忽略大小写宽字符串比较

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  int wcscasecmp(const wchar_t *l, const wchar_t *r);  // 本模块导出的函数签名
```
