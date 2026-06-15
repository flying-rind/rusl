# ttyname.c 规约

> musl libc POSIX 终端名称获取函数。`ttyname` 在 `<unistd.h>` 中声明。非线程安全版本，返回指向内部静态缓冲区的指针。

---

## 依赖图

```
ttyname (Public)
  └── ttyname_r(fd, buf, sizeof buf) — 线程安全版本的终端名获取
        ├── isatty(fd) — 测试是否为终端
        ├── __procfdname(procname, fd) — 构造 /proc/self/fd/<fd> 路径
        ├── readlink(procname, name, size) — 读取符号链接目标
        ├── stat(name, &st1) — 获取终端设备文件状态
        └── fstat(fd, &st2) — 获取文件描述符状态
  └── errno — 保存 ttyname_r 的错误码
```

---

## 函数规约

### ttyname

```c
char *ttyname(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取文件描述符 `fd` 所关联的终端设备路径名。将线程安全版本 `ttyname_r` 的结果存入内部静态缓冲区，提供简化的非线程安全接口。

#### 前置条件

- `fd`: 有效的已打开文件描述符

#### 后置条件

- **Case 1 `fd` 关联终端设备**
  - 返回指向内部静态缓冲区 `buf` 的指针
  - `buf` 内容为终端设备路径名（如 `/dev/pts/0`）
  - 后续调用 `ttyname` 会覆盖 `buf` 内容

- **Case 2 `fd` 不关联终端设备或发生错误**
  - 返回 `NULL`
  - `errno` 设置为 `ttyname_r` 返回的错误码（如 `ENOTTY`、`EBADF`、`ENODEV`、`ERANGE`）

#### 系统算法

```
ttyname(fd):
  static char buf[TTY_NAME_MAX]          // TTY_NAME_MAX = 32
  result = ttyname_r(fd, buf, sizeof buf) // 调用线程安全版本
  if result != 0:
    errno = result                        // 将错误码写入 errno
    return NULL
  else:
    return buf                            // 返回静态缓冲区指针
```

即：
1. 声明静态缓冲区 `buf`，大小为 `TTY_NAME_MAX`（32 字节）
2. 调用 `ttyname_r(fd, buf, sizeof buf)` 获取终端名
3. 若 `ttyname_r` 返回非零（错误码），将该错误码写入 `errno` 并返回 `NULL`
4. 若成功，返回指向静态缓冲区的指针

#### 依赖

- `ttyname_r` — 线程安全版终端名获取（`src/unistd/ttyname_r.c`）
- `TTY_NAME_MAX` — 终端名称最大长度宏，值为 `32`，定义在 `<limits.h>`
- `errno` — POSIX 错误码全局变量
