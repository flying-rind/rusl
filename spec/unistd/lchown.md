# lchown.c 规约

> musl libc POSIX 改变符号链接自身所有者系统调用封装。`lchown` 在 `<unistd.h>` 中声明。

---

## 函数规约

### lchown

```c
int lchown(const char *path, uid_t uid, gid_t gid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

改变符号链接自身（而非其目标）的所有者和/或所属组。与 `chown` 不同，`lchown` 不跟随符号链接。

#### 前置条件

- `path`: 以 NULL 结尾的有效路径
- `uid` / `gid`: 新所有者/组（-1 保持不变）

#### 后置条件

同 `chown`，但操作对象是符号链接自身。

#### 系统算法

```
lchown(path, uid, gid):
  #ifdef SYS_lchown:
    return syscall(SYS_lchown, path, uid, gid)
  #else:
    return syscall(SYS_fchownat, AT_FDCWD, path, uid, gid, AT_SYMLINK_NOFOLLOW)
  #endif
```

#### 依赖

- `SYS_lchown` — Linux 内核系统调用（部分架构）
- `SYS_fchownat` + `AT_SYMLINK_NOFOLLOW` — 回退方案
