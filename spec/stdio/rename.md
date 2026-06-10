# rename.c 规约

> musl libc 标准库文件重命名函数实现。将文件或目录从旧路径重命名为新路径。

---

## 依赖图

```
rename (Public)
  ├── syscall(SYS_rename, ...)      — 重命名系统调用 (首选, syscall.h)
  ├── syscall(SYS_renameat, ...)    — 相对路径重命名系统调用 (fallback, syscall.h)
  └── syscall(SYS_renameat2, ...)   — 增强重命名系统调用 (fallback, syscall.h)
```

---

## 函数规约

### 1. rename

```c
int rename(const char *old, const char *new);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

将文件系统对象从 `old` 路径重命名为 `new` 路径。若 `new` 已存在且为文件，则被原子替换。若 `old` 和 `new` 位于同一文件系统，操作为原子的；在不同文件系统之间通常失败（`EXDEV`）。

#### 前置条件

- `old`: 非空指针，指向以 `\0` 结尾的现有文件路径
- `new`: 非空指针，指向以 `\0` 结尾的目标路径；父目录必须存在
- 调用进程对 `old` 和 `new` 的父目录具有写权限和搜索权限

#### 后置条件

- **Case 1 成功重命名**
  - `old` 路径不再指向该对象
  - 该对象现在由 `new` 路径引用
  - 返回 `0`
  - 不设置 `errno`

- **Case 2 重命名失败（原因包括但不限于）**
  - `old` 不存在 → `errno = ENOENT`
  - 权限不足 → `errno = EACCES` 或 `EPERM`
  - 跨文件系统重命名目录 → `errno = EXDEV`
  - `new` 为已存在目录但 `old` 为空目录 → `errno = EISDIR` 或 `ENOTEMPTY`
  - 返回 `-1`

#### 系统算法

```
rename(old, new):
  1. 若系统支持 SYS_rename:
       return syscall(SYS_rename, old, new)
     否则若支持 SYS_renameat:
       return syscall(SYS_renameat, AT_FDCWD, old, AT_FDCWD, new)
     否则:
       return syscall(SYS_renameat2, AT_FDCWD, old, AT_FDCWD, new, 0)
```

注意：`syscall(...)` 宏内部已包含 `__syscall(...)` 和 `__syscall_ret(...)`，自动完成错误码转换。

#### 不变量

- 同一文件系统内的重命名保证原子性
- 若 `new` 已存在，操作完成后原 `new` 文件不再存在（被原子替换）

#### 依赖

- `syscall(...)` — 系统调用宏，自动进行 `__syscall_ret` 转换（定义于 `src/internal/syscall.h`）
- `SYS_rename` / `SYS_renameat` / `SYS_renameat2` — 系统调用号（来自 `<sys/syscall.h>` 或内核头文件）
- `AT_FDCWD` — 当前工作目录常量（来自 `<fcntl.h>`）

#### 编译时变体

| 优先级 | 宏条件 | 使用的系统调用 | 参数 |
|--------|--------|---------------|------|
| 1 | `SYS_rename` 已定义 | `SYS_rename` | `old, new` |
| 2 | `SYS_renameat` 已定义 | `SYS_renameat` | `AT_FDCWD, old, AT_FDCWD, new` |
| 3 | 默认 fallback | `SYS_renameat2` | `AT_FDCWD, old, AT_FDCWD, new, 0` |
