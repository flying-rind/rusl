# remove.c 规约

> musl libc 标准库文件删除函数实现。删除指定路径的文件或空目录。

---

## 依赖图

```
remove (Public)
  ├── __syscall(SYS_unlink, ...)          — 删除文件系统调用 (syscall.h)
  ├── __syscall(SYS_unlinkat, ...)        — 相对路径删除系统调用 (fallback, syscall.h)
  ├── __syscall(SYS_rmdir, ...)           — 删除目录系统调用 (syscall.h)
  └── __syscall_ret(r)                    — 将内核返回值转换为 libc 返回值 (syscall.h)
```

---

## 函数规约

### 1. remove

```c
int remove(const char *path);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

从文件系统中删除 `path` 指向的文件或空目录。先尝试作为文件删除，若因 `EISDIR`（目标为目录）失败则尝试作为空目录删除。等价于对文件调用 `unlink()`，对目录调用 `rmdir()`。

#### 前置条件

- `path`: 非空指针，指向以 `\0` 结尾的有效路径字符串
- 调用进程对 `path` 所在目录具有写权限和搜索权限
- 若 `path` 指向目录，该目录必须为空

#### 后置条件

- **Case 1 成功删除文件或空目录**
  - `path` 对应的文件/目录从文件系统中移除
  - 返回 `0`
  - 不设置 `errno`

- **Case 2 删除失败（原因包括但不限于）**
  - 路径不存在 → `errno = ENOENT`
  - 权限不足 → `errno = EACCES` 或 `EPERM`
  - 目录非空 → `errno = ENOTEMPTY`
  - `path` 为正在使用的文件 → `errno = EBUSY`
  - 返回 `-1`

#### 系统算法

```
remove(path):
  1. 若系统支持 SYS_unlink:
       r = __syscall(SYS_unlink, path)
     否则:
       r = __syscall(SYS_unlinkat, AT_FDCWD, path, 0)
  
  2. 若 r == -EISDIR (目标为目录):
       若系统支持 SYS_rmdir:
         r = __syscall(SYS_rmdir, path)
       否则:
         r = __syscall(SYS_unlinkat, AT_FDCWD, path, AT_REMOVEDIR)
  
  3. return __syscall_ret(r)  // 将负错误码转换为 -1 + 设置 errno
```

#### 不变量

- 若 `path` 为目录，仅当目录为空时删除成功
- 不可逆操作：文件一旦删除无法通过 libc 恢复

#### 依赖

- `__syscall(...)` — 直接系统调用宏（定义于 `src/internal/syscall.h`）
- `__syscall_ret(...)` — 将内核负错误码转换为用户空间返回值（定义于 `src/internal/syscall.h`）
- `SYS_unlink` / `SYS_unlinkat` / `SYS_rmdir` — 系统调用号（来自 `<sys/syscall.h>` 或内核头文件）
- `AT_FDCWD` — 当前工作目录表示常量（来自 `<fcntl.h>`）
- `AT_REMOVEDIR` — 删除目录标志（来自 `<fcntl.h>`）
- `EISDIR` — 错误码：目标为目录（来自 `<errno.h>`）

#### 编译时变体

| 宏条件 | 第一路径 (文件删除) | 第二路径 (目录删除) |
|--------|--------------------|--------------------|
| `SYS_unlink` 已定义, `SYS_rmdir` 已定义 | `__syscall(SYS_unlink, path)` | `__syscall(SYS_rmdir, path)` |
| `SYS_unlink` 已定义, `SYS_rmdir` 未定义 | `__syscall(SYS_unlink, path)` | `__syscall(SYS_unlinkat, AT_FDCWD, path, AT_REMOVEDIR)` |
| `SYS_unlink` 未定义, `SYS_rmdir` 已定义 | `__syscall(SYS_unlinkat, AT_FDCWD, path, 0)` | `__syscall(SYS_rmdir, path)` |
| 两者均未定义 | `__syscall(SYS_unlinkat, AT_FDCWD, path, 0)` | `__syscall(SYS_unlinkat, AT_FDCWD, path, AT_REMOVEDIR)` |
