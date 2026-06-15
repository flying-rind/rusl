# ttyname_r.c 规约

> musl libc POSIX 终端名称获取函数（线程安全版）。`ttyname_r` 在 `<unistd.h>` 中声明。通过读取 `/proc/self/fd/<fd>` 符号链接并验证设备 ID/inode 一致性来获取终端名。

---

## 依赖图

```
ttyname_r (Public)
  ├── isatty(fd) — 测试是否为终端
  ├── __procfdname(procname, fd) — 构造 /proc/self/fd/<fd> 路径
  ├── readlink(procname, name, size) — 读取符号链接目标
  ├── stat(name, &st1) — 获取终端设备文件状态
  └── fstat(fd, &st2) — 获取文件描述符状态
```

---

## 函数规约

### ttyname_r

```c
int ttyname_r(int fd, char *name, size_t size);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取文件描述符 `fd` 所关联的终端设备路径名，写入用户提供的缓冲区 `name`。实现过程：
1. 首先通过 `isatty` 确认 `fd` 是否为终端
2. 通过 `/proc/self/fd/<fd>` 符号链接获取内核记录的路径
3. 读取符号链接目标到 `name` 缓冲区
4. 通过 `stat`/`fstat` 验证设备 ID 和 inode 一致性，防止 `/proc` 挂载点下路径伪造（TOCTOU 保护）

#### 前置条件

- `fd`: 有效的已打开文件描述符
- `name`: 指向至少 `size` 字节可用内存的非空指针
- `size`: 缓冲区大小，`> 0`

#### 后置条件

- **Case 1 成功**
  - `name` 缓冲区被写入以 null 结尾的终端设备路径名（如 `/dev/pts/0`）
  - 终端设备通过 stat/fstat 验证（设备 ID 和 inode 一致）
  - 返回 0

- **Case 2 `fd` 不是终端**
  - 返回 `errno`（由 `isatty` 设置，如 `ENOTTY`、`EBADF`）

- **Case 3 读取符号链接失败**
  - 返回 `errno`（如 `ENOENT`、`EACCES`）

- **Case 4 缓冲区太小**
  - 返回 `ERANGE`
  - `name` 内容未定义

- **Case 5 设备验证失败（TOCTOU 保护）**
  - `stat` 或 `fstat` 失败：返回 `errno`
  - 设备 ID 或 inode 不匹配：返回 `ENODEV`

#### 系统算法

```
ttyname_r(fd, name, size):
  struct stat st1, st2
  char procname[sizeof "/proc/self/fd/" + 3*sizeof(int) + 2]

  if !isatty(fd):                          // 1. 不是终端，直接返回 errno
    return errno

  __procfdname(procname, fd)               // 2. 构造 "/proc/self/fd/<fd>" 字符串

  l = readlink(procname, name, size)       // 3. 读取符号链接目标长度

  if l < 0: return errno                   // 4a. readlink 失败
  if l == size: return ERANGE              // 4b. 缓冲区太小（无法 null 结尾）
  name[l] = 0                              // 4c. 添加 null 终止符

  if stat(name, &st1) || fstat(fd, &st2):  // 5a. stat/fstat 任一失败
    return errno

  if st1.st_dev != st2.st_dev ||           // 5b. 设备 ID 不匹配
     st1.st_ino != st2.st_ino:             //     或 inode 不匹配
    return ENODEV

  return 0                                 // 6. 验证通过，成功
```

#### 依赖

- `isatty` — 终端检测函数（`src/unistd/isatty.c`）
- `__procfdname` — 构造 `/proc/self/fd/<fd>` 路径字符串（`src/internal/procfdname.c`，`hidden` 内部函数）
- `readlink` — 系统调用，读取符号链接目标（`src/unistd/readlink.c`）
- `stat` — 系统调用，获取文件状态（`src/stat/stat.c`）
- `fstat` — 系统调用，通过 fd 获取文件状态（`src/stat/fstat.c`）
- `struct stat` — 文件状态结构体，定义在 `<sys/stat.h>`
- `ERANGE` — 缓冲区大小不足错误码，定义在 `<errno.h>`
- `ENODEV` — 设备不匹配错误码，定义在 `<errno.h>`

#### 安全说明

当前实现存在已知缺陷：`readlink` 返回路径后、`stat`/`fstat` 验证前存在竞争窗口。在此期间终端可能被解除分配并重新分配给其他用户，导致信息泄露。此问题在 POSIX 标准中有记录。
