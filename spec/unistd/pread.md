# pread.c 规约

> musl libc POSIX 指定偏移读取系统调用封装。`pread` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
pread (Public)
  └── syscall_cp(SYS_pread, fd, buf, size, __SYSCALL_LL_PRW(ofs))
        ├── __syscall_cp(...) — 可被信号取消的原始系统调用
        └── __syscall_ret(...) — 返回值转换
```

---

## 函数规约

### pread

```c
ssize_t pread(int fd, void *buf, size_t size, off_t ofs);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

从文件描述符 `fd` 的指定偏移 `ofs` 处读取至多 `size` 字节到缓冲区 `buf`，文件偏移指针不变。等效于 `lseek(fd, ofs, SEEK_SET)` + `read(fd, buf, size)` + 恢复原偏移的原子操作。

#### 前置条件

- `fd`: 有效的已打开文件描述符（必须支持定位操作）
- `buf`: 指向至少 `size` 字节可用内存的非空指针
- `size`: `[0, SSIZE_MAX]` 范围内的字节数
- `ofs`: 文件中的有效偏移位置

#### 后置条件

- **Case 1 成功**
  - 返回实际读取的字节数（0 表示 EOF）
  - `fd` 的当前文件偏移不变

- **Case 2 错误**
  - 返回 -1
  - `errno` 设置为 `EBADF`、`ESPIPE`（不可定位 fd）等

#### 系统算法

```
pread(fd, buf, size, ofs):
  return syscall_cp(SYS_pread, fd, buf, size, __SYSCALL_LL_PRW(ofs))
```

其中 `__SYSCALL_LL_PRW(ofs)` 在 32 位平台上将 `off_t`（64 位）展开为两个 long 参数（高 32 位在前），在 64 位平台上直接使用 ofs。

#### 依赖

- `SYS_pread` — Linux 内核系统调用
- `__SYSCALL_LL_PRW` — 偏移量 32/64 位适配宏
