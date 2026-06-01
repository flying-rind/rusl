# wcstok

## 函数签名
```c
wchar_t *wcstok(wchar_t *restrict s, const wchar_t *restrict sep, wchar_t **restrict p);
```

## 意图
从宽字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态。

## 前置条件
- `p != NULL`
- 首次调用时 `s != NULL`；后续调用可传 `s == NULL`
- `sep != NULL`
- sep 指向以 L'\0' 结尾的有效宽字符串
- 当 `s != NULL` 时，s 指向可修改的以 L'\0' 结尾的有效宽字符串

## 后置条件
- 若无更多 token，返回值为 NULL，且 *p = 0
- 若有 token：返回值为指向 token 起始位置的指针，且分隔符位置被 L'\0' 替换
- *p 被更新为下一个搜索位置

## 不变量
- *p 始终为 NULL（无更多 token）或指向下一个搜索起始位置

## 算法
1. 若 s == NULL，使用 *p 续接
2. 使用 wcsspn 跳过前导分隔符
3. 若到达 L'\0'，设置 *p=0 返回 NULL
4. 使用 wcscspn 找到分隔符位置
5. 若找到，用 L'\0' 替代，递增 *p
6. 返回 token 起始位置

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcsspn(const wchar_t *s, const wchar_t *c);  // 依赖1: 跳过前导分隔符
  size_t wcscspn(const wchar_t *s, const wchar_t *c);  // 依赖2: 找到下一个分隔符位置

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcstok(wchar_t *restrict s, const wchar_t *restrict sep, wchar_t **restrict p);
