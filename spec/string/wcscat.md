# wcscat

## 函数签名
```c
wchar_t *wcscat(wchar_t *restrict dest, const wchar_t *restrict src);
```

## 意图
将 src 宽字符串追加到 dest 宽字符串末尾（覆盖 dest 的终止 L'\0'）。

## 前置条件
- `dest != NULL`
- `src != NULL`
- `dest` 和 `src` 不重叠（restrict 约束）
- dest 指向以 L'\0' 结尾的有效宽字符串
- src 指向以 L'\0' 结尾的有效宽字符串
- dest 指向的缓冲区至少可容纳 (wcslen(dest) + wcslen(src) + 1) 个 wchar_t

## 后置条件
- dest[dest_len..dest_len+src_len] == src[0..src_len]
- dest[dest_len+src_len] == L'\0'
- 返回值为 dest

## 不变量
- 无全局或静态状态被修改

## 算法
1. 通过 wcslen(dest) 定位 dest 末尾的 L'\0'
2. 使用 wcscpy 将 src 复制到该位置
3. 返回 dest

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcslen(const wchar_t *s);                                     // 依赖1: 宽字符串长度计算
  wchar_t *wcscpy(wchar_t *restrict d, const wchar_t *restrict s);    // 依赖2: 宽字符串复制

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  wchar_t *wcscat(wchar_t *restrict dest, const wchar_t *restrict src);  // 本模块导出的函数签名
```
