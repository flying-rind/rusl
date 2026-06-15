# fchown.c 规约

> musl libc POSIX 通过文件描述符改变文件所有者系统调用封装。`fchown` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
fchown (Public)
  ├── __syscall(SYS_fchown, fd, uid, gid)
  │     └── 非 -EBADF: __syscall_ret(ret)
  │     └── -EBADF: 检查 fd 有效性
  │           ├── __syscall(SYS_fcntl, fd, F_GETFD) < 0: fd 真的无效 → 返回 EBADF
  │           └── fd 有效 → 通过 /proc/self/fd/<fd> 回退
  │                 ├── __procfdname(buf, fd)
  │                 └── 通过路径 chown/fchownat
  └── [回退] 通过 /proc/self/fd/<fd> 路径修改所有者
```

---

## 函数规约

### fchown

```c
int fchown(int fd, uid_t uid, gid_t gid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

通过文件描述符 `fd` 改变文件的所有者和/或所属组。实现了与 `fchdir` 类似的 /proc 回退逻辑。

#### 前置条件

- `fd`: 有效的已打开文件描述符
- `uid` / `gid`: 新所有者/组（-1 保持不变）

#### 后置条件

- **Case 1 成功**: 返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
fchown(fd, uid, gid):
  ret = __syscall(SYS_fchown, fd, uid, gid)
  if ret != -EBADF: return __syscall_ret(ret)

  if __syscall(SYS_fcntl, fd, F_GETFD) < 0: return __syscall_ret(ret)

  __procfdname(buf, fd)                                    // "/proc/self/fd/<fd>"
  #ifdef SYS_chown:
    return syscall(SYS_chown, buf, uid, gid)
  #else:
    return syscall(SYS_fchownat, AT_FDCWD, buf, uid, gid, 0)
  #endif
```

#### 依赖

- `SYS_fchown` — Linux 内核系统调用
- `__procfdname` — 构造 /proc 路径
- `SYS_fcntl` + `F_GETFD` — 检查 fd 有效性
- `SYS_chown` / `SYS_fchownat` — 回退路径
