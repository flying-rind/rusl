# lseek.c 规约

> musl libc POSIX 文件定位系统调用封装。`__lseek` 是 musl 内部实现，`lseek` 是其弱别名。

---

## 依赖图

```
__lseek (Internal, 导出为 lseek 的弱别名)
lseek (Public, weak_alias of __lseek)
  ├── [SYS__llseek 存在时] syscall(SYS__llseek, fd, off_hi, off_lo, &result, whence)
  │     └── 若返回非零则 return -1，否则 return result
  └── [SYS__llseek 不存在时] syscall(SYS_lseek, fd, offset, whence)
```

---

## 函数规约

### __lseek (lseek)

```c
off_t __lseek(int fd, off_t offset, int whence);
// weak_alias(__lseek, lseek);
off_t lseek(int fd, off_t offset, int whence);
```

[Visibility]:
- `__lseek`: Internal (不导出) — musl 内部实现，供其他内部模块调用
- `lseek`: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

重新定位文件描述符 `fd` 的读写位置（文件偏移）。对于普通文件，新位置由 `offset` 和 `whence` 决定；对于管道、套接字等不可定位的文件描述符则返回错误。

#### 前置条件

- `fd`: 有效的已打开文件描述符
- `whence`: `SEEK_SET` (0)、`SEEK_CUR` (1)、`SEEK_END` (2)、`SEEK_DATA` (3) 或 `SEEK_HOLE` (4)
- `offset`: 文件偏移量（可与 `off_t` 范围匹配的有符号值）

#### 后置条件

- **Case 1 成功**
  - 返回新的文件偏移位置（从文件开头的字节偏移，非负值）

- **Case 2 错误**
  - 返回 -1（`(off_t)-1`）
  - `errno` 设置为 `EBADF`（fd 无效）、`EINVAL`（whence 无效或产生的偏移为负）、`ESPIPE`（fd 是管道/套接字）、`EOVERFLOW`（结果超出 off_t 范围）等

#### 系统算法

```
__lseek(fd, offset, whence):
  #ifdef SYS__llseek:                 // 32 位平台
    off_t result
    ret = syscall(SYS__llseek, fd, offset>>32, offset, &result, whence)
    return ret ? -1 : result          // 错误时返回 -1
  #else:                              // 64 位平台
    return syscall(SYS_lseek, fd, offset, whence)
  #endif
```

注意：32 位平台上 `SYS__llseek` 使用独立的高/低 32 位参数和结果指针，支持 64 位偏移。

#### 依赖

- `SYS_lseek` — Linux 内核系统调用（64 位平台）
- `SYS__llseek` — Linux 内核扩展 llseek 系统调用（32 位平台）
