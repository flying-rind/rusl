# \_\_strerror.h 规约

> musl libc 内部错误消息数据定义头文件。通过宏 `E(n, s)` 枚举所有标准 `errno` 值及其对应的可读错误消息字符串。

---

## 宏协议

### E(n, s)

```c
E(n, s)
```

[Visibility]: Internal — musl 内部宏数据定义，仅在 `strerror.c` 中通过 `#define`-`#include`-`#undef` 模式多次包含使用

#### Intent

定义错误码 `n` 到错误描述字符串 `s` 的映射关系。本文件被 `strerror.c` 包含三次，每次通过不同的 `E` 宏定义分别生成：

1. **结构体成员声明** (`#define E(n, s) char str##n[sizeof(s)];`) — 为每个错误码声明固定大小的字符数组
2. **结构体初始化** (`#define E(n, s) s,`) — 用错误消息字符串初始化结构体
3. **索引表定义** (`#define E(n, s) [n] = offsetof(struct errmsgstr_t, str##n),`) — 构建从错误码到偏移量的查找表

#### 包含条目

共约 60+ 条错误码映射，涵盖：

- **C 标准错误**: `EDOM`, `ERANGE`, `EILSEQ`
- **POSIX 基础错误**: `EACCES`, `EPERM`, `ENOENT`, `EINVAL`, `ENOMEM`, `EIO` 等
- **文件系统错误**: `EROFS`, `ENOTEMPTY`, `EXDEV`, `EFBIG`, `EMLINK` 等
- **网络错误**: `ECONNREFUSED`, `ETIMEDOUT`, `EHOSTUNREACH`, `EADDRINUSE` 等
- **线程/同步错误**: `EDEADLK`, `ECANCELED`, `EOWNERDEAD` 等
- **Linux 扩展错误**: `EKEYEXPIRED`, `EKEYREVOKED`, `ENOMEDIUM` 等

#### 特殊条目

- `E(0, "No error information")` — 通用/未知错误的回退消息，必须为第一条

#### 存储影响

- 最终生成的 `errmsgstr` 结构体大小约为所有错误消息字符串长度之和
- 索引表 `errmsgidx` 共有 `MAX_ERRNO + 1` 个 `unsigned short` 条目（2 字节/条目）

#### 依赖

无。
