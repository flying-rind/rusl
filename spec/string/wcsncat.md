# wcsncat

## 函数签名
```c
wchar_t *wcsncat(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

## 意图
将 s 中最多 n 个宽字符追加到 d 末尾，始终在末尾添加 L'\0'。

## 前置条件
- `d != NULL`
- `s != NULL`
- `d` 和 `s` 不重叠（restrict 约束）
- d 指向以 L'\0' 结尾的有效宽字符串
- s 指向以 L'\0' 结尾的有效宽字符串
- d 指向的缓冲区至少可容纳 (wcslen(d) + min(n, wcslen(s)) + 1) 个 wchar_t

## 后置条件
- d 末尾追加了 s 的前 min(n, wcslen(s)) 个宽字符以及 L'\0'
- d 原有内容不变（除追加部分外）
- 返回值为 d

## 不变量
- 最多写入 min(n, wcslen(s)) + 1 个宽字符

## 算法
1. 将 d 移到 wcslen(d) 处（L'\0' 位置）
2. 循环复制最多 n 个非 L'\0' 宽字符
3. 追加终止 L'\0'
4. 返回原保存的 a

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t wcslen(const wchar_t *s);  // 依赖1: 计算目标字符串长度以定位末尾

Predefined Macros:
  (无外部宏依赖)

[GUARANTEE]
Exported Interface:
  wchar_t *wcsncat(wchar_t *restrict d, const wchar_t *restrict s, size_t n);  // 追加最多n个宽字符到字符串末尾
