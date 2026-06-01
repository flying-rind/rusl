# strerror_r

## 函数签名
```c
int strerror_r(int err, char *buf, size_t buflen);
```

## 意图
将错误码 err 对应的错误描述字符串安全地复制到用户提供的缓冲区 buf 中（线程安全版本）。

## 前置条件
- `buf != NULL` 或 `buflen == 0`
- 当 `buflen > 0` 时，`buf` 指向的缓冲区至少可写入 buflen 个字节

## 后置条件
- 返回值为 0：成功，buf 中包含完整的错误消息（null 终止）
- 返回值为 ERANGE：缓冲区不足；若 buflen > 0，buf 中包含截断的错误消息（null 终止）
- 若 buflen == 0 且消息长度 >= buflen，buf 不被写入

## 不变量
- buf 末尾始终被正确终止（当 buflen > 0 时）
- 最多写入 buflen 个字节到 buf，绝不会溢出

## 算法
1. 调用 strerror(err) 获取错误消息指针
2. 计算消息长度 l
3. 若 l >= buflen（缓冲区不足）：
   - 若有空间（buflen > 0），复制 buflen-1 字节并以 '\0' 终止
   - 返回 ERANGE
4. 否则复制完整消息（包括 '\0'），返回 0

/* Rely */
[RELY]
Predefined Structures/Functions:
  size_t;  // 依赖1: 缓冲区长度类型
  strerror(int err);  // 依赖2: 获取错误码对应的错误消息字符串
  strlen(const char *s);  // 依赖3: 计算错误消息长度
  memcpy(void *dest, const void *src, size_t n);  // 依赖4: 复制错误消息到用户缓冲区

Predefined Macros:
  ERANGE;  // 依赖1: 位于 <errno.h>，缓冲区不足错误码
  weak_alias(old, new);  // 依赖2: 内部宏，用于创建弱符号别名

[GUARANTEE]
Exported Interface:
  int strerror_r(int err, char *buf, size_t buflen);  // 线程安全地获取错误描述字符串
  int __xpg_strerror_r(int err, char *buf, size_t buflen);  // strerror_r 的 XPG 标准别名
