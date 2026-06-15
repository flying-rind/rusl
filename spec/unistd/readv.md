# readv.c 规约

> musl libc POSIX 分散读取系统调用封装。`readv` 在 `<sys/uio.h>` 中声明。

---

## 依赖图

```
readv (Public)
  └── syscall_cp(SYS_readv, fd, iov, count)
```

---

## 函数规约

### readv

```c
ssize_t readv(int fd, const struct iovec *iov, int count);
```

[Visibility]: User — `<sys/uio.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

从文件描述符 `fd` 读取数据到多个不连续的缓冲区（`iov` 数组），原子操作，等效于单次 `read` 到拼接缓冲区但无需数据拷贝。

#### 前置条件

- `fd`: 有效可读文件描述符
- `iov`: 指向 `count` 个 `struct iovec` 的非空指针，每个元素含 `.iov_base`（缓冲区指针）和 `.iov_len`（缓冲区大小）
- `count`: `[0, IOV_MAX]` 范围内的向量数量

#### 后置条件

- **Case 1 成功**: 返回实际读取字节数（可能小于总缓冲区大小）
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
readv(fd, iov, count):
  return syscall_cp(SYS_readv, fd, iov, count)
```

#### 依赖

- `SYS_readv` — Linux 内核系统调用
- `struct iovec` — `<sys/uio.h>`
