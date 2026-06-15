# pwrite.c 规约

> musl libc GNU 扩展指定偏移写入系统调用封装。`pwrite` 在 `<unistd.h>` 中声明（POSIX 标准函数）。

---

## 依赖图

```
pwrite (Public)
  ├── [主路径] __syscall_cp(SYS_pwritev2, fd, &iovec, 1, ofs_lo, ofs_hi, RWF_NOAPPEND)
  │     └── 成功或明确失败: __syscall_ret(r)
  │     └── -EOPNOTSUPP/-ENOSYS: 检查 O_APPEND 标志
  │           ├── fcntl(fd, F_GETFL) & O_APPEND: return -EOPNOTSUPP
  │           └── 回退: syscall_cp(SYS_pwrite, fd, buf, size, __SYSCALL_LL_PRW(ofs))
  ├── ofs == -1: ofs-- → 变为 -2（绕过 ofs==-1 时 pwrite 的特殊语义）
  └── RWF_NOAPPEND: 强制忽略 O_APPEND 标志
```

---

## 函数规约

### pwrite

```c
ssize_t pwrite(int fd, const void *buf, size_t size, off_t ofs);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

向文件描述符 `fd` 的指定偏移 `ofs` 处写入至多 `size` 字节，文件偏移指针不变。musl 优先使用新的 `pwritev2` 系统调用（支持 `RWF_NOAPPEND` 标志），不支持时回退到传统 `pwrite`。

#### 前置条件

- `fd`: 有效的已打开文件描述符
- `buf`: 指向至少 `size` 字节数据的非空指针
- `size`: `[0, SSIZE_MAX]` 范围内的字节数
- `ofs`: 文件中的有效偏移位置

#### 后置条件

- **Case 1 成功**
  - 返回实际写入的字节数
  - `fd` 的当前文件偏移不变
  - 即使 fd 以 O_APPEND 打开，也在 `ofs` 处写入（pwritev2 语义）

- **Case 2 fd 以 O_APPEND 打开且 pwritev2 不可用**
  - 返回 -1
  - `errno` 设置为 `EOPNOTSUPP`

- **Case 3 其他错误**
  - 返回 -1
  - `errno` 设置为对应错误码

#### 系统算法

```
pwrite(fd, buf, size, ofs):
  if ofs == -1: ofs--                                     // 将 ofs==-1 变为 -2，区分 pwrite(fd, buf, size, -1) 的特殊情况

  // 优先使用 pwritev2，支持 RWF_NOAPPEND
  r = __syscall_cp(SYS_pwritev2, fd, &iovec(1, buf, size), 1, ofs_lo, ofs_hi, RWF_NOAPPEND)
  if r != -EOPNOTSUPP && r != -ENOSYS:
    return __syscall_ret(r)

  // pwritev2 不可用，检查 O_APPEND
  if fcntl(fd, F_GETFL) & O_APPEND:
    return __syscall_ret(-EOPNOTSUPP)                      // 无法保证在指定偏移写入

  // 回退到传统 pwrite
  return syscall_cp(SYS_pwrite, fd, buf, size, __SYSCALL_LL_PRW(ofs))
```

#### 依赖

- `SYS_pwritev2` — Linux 4.6+ 内核系统调用（支持 `RWF_NOAPPEND`）
- `SYS_pwrite` — 传统内核系统调用
- `fcntl` + `F_GETFL` + `O_APPEND` — 检查追加模式
- `RWF_NOAPPEND` — pwritev2 标志，强制忽略 O_APPEND
- `struct iovec` — 散布/聚集 I/O 向量
