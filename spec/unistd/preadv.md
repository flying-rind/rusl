# preadv.c 规约

> musl libc BSD 扩展指定偏移分散读取系统调用封装。`preadv` 在 `<sys/uio.h>` 中声明（`_BSD_SOURCE` / `_GNU_SOURCE`）。

---

## 依赖图

```
preadv (Public [BSD/GNU])
  └── syscall_cp(SYS_preadv, fd, iov, count, ofs_lo, ofs_hi)
```

---

## 函数规约

### preadv

```c
ssize_t preadv(int fd, const struct iovec *iov, int count, off_t ofs);
```

[Visibility]: User — BSD/GNU 扩展函数，定义 `_BSD_SOURCE` 或 `_GNU_SOURCE` 后可用

#### Intent

从文件描述符 `fd` 的指定偏移 `ofs` 处分散读取数据到多个缓冲区，不改变当前文件偏移。等效于 `lseek + readv + lseek` 的原子操作。

#### 前置条件

- `fd`: 有效、支持定位的文件描述符
- `iov`: 指向 `count` 个 `struct iovec` 的非空指针
- `count`: `[0, IOV_MAX]` 范围内
- `ofs`: 文件偏移

#### 后置条件

- **Case 1 成功**: 返回实际读取字节数，文件偏移不变
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
preadv(fd, iov, count, ofs):
  return syscall_cp(SYS_preadv, fd, iov, count, (long)(ofs), (long)(ofs>>32))
```

32 位平台上 `off_t`（64 位）被拆分为两个 `long` 参数传递（低 32 位在前，高 32 位在后）。

#### 依赖

- `SYS_preadv` — Linux 内核系统调用
