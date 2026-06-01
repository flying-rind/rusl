# snprintf

## 函数签名
```c
int snprintf(char *restrict s, size_t n, const char *restrict fmt, ...);
```

## 意图
格式化字符串到固定大小缓冲区，保证不溢出且 null 终止。是 `vsnprintf` 的可变参数包装。

## 前置条件
- `fmt != NULL`，指向有效格式化字符串
- 若 `n > 0`，则 `s` 指向至少 `n` 字节的可写内存; 若 `n == 0`，则 `s` 可为 `NULL`

## 后置条件
- 返回值 `r` 为格式化完成后字符串的总长度（不含 `'\0'`）
- 若 `r >= 0` 且 `n > 0`，则 `s[0..min(r, n-1)]` 为截断后的输出，`s[min(r, n-1)] == '\0'`
- 若 `r < 0`，则发生了编码错误
- `ap`（`va_list`）在返回前已被 `va_end` 清理

## 不变量
无。本函数纯粹作为转发代理。

## 算法
1. `va_start(ap, fmt)` 初始化可变参数列表
2. 委托 `vsnprintf(s, n, fmt, ap)` 完成所有格式化与写入逻辑
3. `va_end(ap)` 清理
4. 返回 `vsnprintf` 的返回值

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vsnprintf(char *restrict s, size_t n, const char *restrict fmt, va_list ap);
                                   // 依赖1: 核心格式化引擎
Predefined Macros:
  va_start / va_end / va_list     // 依赖2: C 标准可变参数宏

[GUARANTEE]
Exported Interface:
  int snprintf(char *restrict s, size_t n, const char *restrict fmt, ...);
                                   // 本模块保证对外提供 snprintf 接口