# vdprintf.c 规约

> musl libc `va_list` 版文件描述符格式化输出函数。通过构造最小 FILE 对象并委托 `vfprintf` 实现。

---

## 依赖图

```
vdprintf
  ├─> 构造 FILE (栈上)
  ├─> vfprintf(f, fmt, ap)  (see vfprintf.c spec)
  └─> __stdio_write  (see stdio_impl.h)
```

---

## 函数规约

### 1. vdprintf

```c
int vdprintf(int fd, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（POSIX 扩展）

#### Intent

将格式化字符串写入文件描述符 `fd`。通过构造一个最小伪 `FILE` 对象绕过 `FILE*` 流机制直接写入文件描述符。

#### 前置条件

- `fd` 为有效的文件描述符
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：返回写入 `fd` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`

#### 系统算法

```
vdprintf(fd, fmt, ap):
  1. 在栈上构造 FILE 对象：
     .fd = fd
     .lbf = EOF (无行缓冲)
     .write = __stdio_write (直接系统调用写入)
     .buf = (void *)fmt (空操作指针，无实际缓冲)
     .buf_size = 0 (无缓冲模式)
     .lock = -1 (禁用锁定，无并发保护)
  2. return vfprintf(&f, fmt, ap)
```

#### 不变量

- `FILE` 对象仅在栈上存在，函数返回后销毁
- 不使用缓冲（`buf_size = 0`），每次 `vfprintf` 的写入直接通过 `__stdio_write` 进入系统调用
- 无锁模式（`lock = -1`），因为伪流不会被多个线程共享

#### 依赖

- `vfprintf()` — 格式化输出核心引擎（见 `vfprintf.c`）
- `__stdio_write()` — 直接文件描述符写入（见 `src/stdio/__stdio_write.c`）
- `stdio_impl.h` — `FILE` 结构体定义
