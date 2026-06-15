# pwritev.c 规约

> musl libc GNU 扩展指定偏移聚集写入系统调用封装。`pwritev` 在 `<sys/uio.h>` 中声明（`_GNU_SOURCE`）。

---

## 依赖图

```
pwritev (Public [GNU])
  ├── [主路径] __syscall_cp(SYS_pwritev2, fd, iov, count, ofs_lo, ofs_hi, RWF_NOAPPEND)
  │     └── 成功或明确失败: __syscall_ret(r)
  │     └── -EOPNOTSUPP/-ENOSYS: 检查 O_APPEND
  │           ├── fcntl(fd, F_GETFL) & O_APPEND: return -EOPNOTSUPP
  │           └── 回退: syscall_cp(SYS_pwritev, fd, iov, count, ofs_lo, ofs_hi)
  └── ofs == -1: ofs-- → 变为 -2（绕过 ofs==-1 的特殊语义）
```

---

## 函数规约

### pwritev

```c
ssize_t pwritev(int fd, const struct iovec *iov, int count, off_t ofs);
```

[Visibility]: User — GNU 扩展函数（`_GNU_SOURCE`），用户程序可直接调用

#### Intent

将多个缓冲区的数据原子性地写入文件描述符 `fd` 的指定偏移 `ofs` 处，文件偏移不变。优先使用 `pwritev2` 系统调用以支持 `RWF_NOAPPEND`，确保即使 fd 以 O_APPEND 打开也能在指定偏移写入。

#### 前置条件

- `fd`: 有效可写文件描述符
- `iov`: 指向 `count` 个 `struct iovec` 的非空指针
- `count`: `[0, IOV_MAX]` 范围内
- `ofs`: 文件偏移（-1 会被调整为 -2 以区分特殊语义）

#### 后置条件

- **Case 1 成功**: 返回实际写入字节数，文件偏移不变
- **Case 2 fd 以 O_APPEND 打开且 pwritev2 不可用**: 返回 -1，errno = `EOPNOTSUPP`
- **Case 3 其他错误**: 返回 -1，设置 errno

#### 系统算法

```
pwritev(fd, iov, count, ofs):
  if ofs == -1: ofs--                                                    // ofs==-1 → -2
  r = __syscall_cp(SYS_pwritev2, fd, iov, count, ofs_lo, ofs_hi, RWF_NOAPPEND)
  if r != -EOPNOTSUPP && r != -ENOSYS: return __syscall_ret(r)           // 成功或明确失败
  if fcntl(fd, F_GETFL) & O_APPEND: return __syscall_ret(-EOPNOTSUPP)    // 无法保证位置写入
  return syscall_cp(SYS_pwritev, fd, iov, count, ofs_lo, ofs_hi)        // 回退
```

#### 依赖

- `SYS_pwritev2` — Linux 4.6+ 系统调用
- `SYS_pwritev` — 传统系统调用
- `fcntl` + `F_GETFL` + `O_APPEND` — 检查追加模式
- `RWF_NOAPPEND` — 忽略 O_APPEND 标志
