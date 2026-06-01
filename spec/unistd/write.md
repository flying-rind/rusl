# write

## 函数签名
```c
ssize_t write(int fd, const void *buf, size_t count);
```

## 意图
将 `buf` 中最多 `count` 字节写入文件描述符 `fd`。本函数是 `SYS_write` 系统调用的线程取消安全点包装，通过 `syscall_cp` 在阻塞前后检查取消请求。

## 前置条件
- `fd` 是已打开且可写的文件描述符
- `buf != NULL`，指向至少 `count` 字节的可读内存

## 后置条件
- Case 1 成功: 返回 `r`（`0 <= r <= count`），表示实际写入的字节数; `buf` 中的前 `r` 字节已提交到 `fd`
- Case 2 失败: 返回 `-1`，`errno` 被设置

## 不变量
无。本函数不持有任何内部状态。

## 算法
直接委托: `syscall_cp(SYS_write, fd, buf, count)` → 返回值原样透传。

/* Rely */
[RELY]
Predefined Structures/Functions:
  ssize_t syscall_cp(long nr, long a1, long a2, long a3);
                                  // 依赖1: 取消点系统调用包装，定义于 syscall.h
Predefined Macros:
  SYS_write                      // 依赖2: 系统调用号，定义于 syscall.h

[GUARANTEE]
Exported Interface:
  ssize_t write(int fd, const void *buf, size_t count);
                                  // 本模块保证对外提供 write 接口