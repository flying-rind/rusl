# isatty.c 规约

> musl libc POSIX 终端检测函数。`isatty` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
isatty (Public)
  └── syscall(SYS_ioctl, fd, TIOCGWINSZ, &wsz)
        ├── __syscall(SYS_ioctl, fd, TIOCGWINSZ, &wsz) — 原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### isatty

```c
int isatty(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

测试文件描述符 `fd` 是否关联到一个终端设备。通过尝试对 `fd` 执行 `TIOCGWINSZ`（获取终端窗口大小）ioctl 系统调用来判断：若调用成功（返回 0），`fd` 即为终端；若调用失败（返回 -1），`fd` 不是终端。利用 "+1" 技巧将错误状态（0/-1）转换为布尔值（1/0）。

#### 前置条件

- `fd`: 有效的已打开文件描述符（无效 fd 会被判定为非终端，返回 0）

#### 后置条件

- **Case 1 `fd` 关联终端设备**
  - 返回 1（true）
  - `errno` 不变

- **Case 2 `fd` 不关联终端设备**
  - 返回 0（false）
  - `errno` 可能被设置（如 `EBADF`、`ENOTTY`），但函数不依赖 errno 做判断

#### 系统算法

```
isatty(fd):
  struct winsize wsz
  return syscall(SYS_ioctl, fd, TIOCGWINSZ, &wsz) + 1
  // syscall 返回 0 (成功)  → +1 = 1 (是终端)
  // syscall 返回 -1 (失败) → +1 = 0 (不是终端)
```

即：
1. 声明 `struct winsize wsz` 作为 ioctl 输出缓冲区（实际不需要读取结果）
2. 调用 `syscall(SYS_ioctl, fd, TIOCGWINSZ, &wsz)` 执行内核 ioctl 系统调用
3. 返回值 +1：0 变为 1（是终端），-1 变为 0（不是终端）

#### 依赖

- `SYS_ioctl` — Linux 内核系统调用编号 (x86_64: 16, aarch64: 29)
- `TIOCGWINSZ` — 获取终端窗口大小的 ioctl 请求码 (`0x5413`)
- `struct winsize` — 终端窗口大小结构体，定义在 `<sys/ioctl.h>`
- `syscall` — 内部宏，定义在 `src/internal/syscall.h`
