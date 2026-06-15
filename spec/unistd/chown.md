# chown.c 规约

> musl libc POSIX 改变文件所有者系统调用封装。`chown` 在 `<unistd.h>` 中声明。

---

## 函数规约

### chown

```c
int chown(const char *path, uid_t uid, gid_t gid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将文件 `path` 的所有者和/或所属组改为 `uid` 和 `gid`。若 `uid` 或 `gid` 为 -1，对应属性保持不变。

#### 前置条件

- `path`: 以 NULL 结尾的有效文件路径
- `uid`: 新所有者 UID（-1 保持不变）或 `gid`: 新组 GID（-1 保持不变）

#### 后置条件

- **Case 1 成功**: 文件所有者/组变更，返回 0
- **Case 2 错误**: 返回 -1，设置 errno（`EPERM`、`ENOENT`、`EACCES` 等）

#### 系统算法

```
chown(path, uid, gid):
  #ifdef SYS_chown:
    return syscall(SYS_chown, path, uid, gid)
  #else:
    return syscall(SYS_fchownat, AT_FDCWD, path, uid, gid, 0)
  #endif
```

#### 依赖

- `SYS_chown` — Linux 内核系统调用（部分架构）
- `SYS_fchownat` + `AT_FDCWD` — 回退方案
