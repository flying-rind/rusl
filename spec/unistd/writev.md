# writev.c 规约

> musl libc POSIX 聚集写入系统调用封装。`writev` 在 `<sys/uio.h>` 中声明。

---

## 依赖图

```
writev (Public)
  └── syscall_cp(SYS_writev, fd, iov, count)
```

---

## 函数规约

### writev

```c
ssize_t writev(int fd, const struct iovec *iov, int count);
```

[Visibility]: User — `<sys/uio.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将多个不连续缓冲区（`iov` 数组）的数据原子性地写入文件描述符 `fd`。等效于单次 `write` 调用但避免数据拼接拷贝。

#### 前置条件

- `fd`: 有效可写文件描述符
- `iov`: 指向 `count` 个 `struct iovec` 的非空指针
- `count`: `[0, IOV_MAX]` 范围内

#### 后置条件

- **Case 1 成功**: 返回实际写入字节数
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
writev(fd, iov, count):
  return syscall_cp(SYS_writev, fd, iov, count)
```

#### 依赖

- `SYS_writev` — Linux 内核系统调用
