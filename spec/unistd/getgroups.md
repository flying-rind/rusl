# getgroups.c 规约

> musl libc POSIX 获取附加组列表系统调用封装。`getgroups` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getgroups (Public)
  └── syscall(SYS_getgroups, count, list)
        ├── __syscall(SYS_getgroups, count, list) — 原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### getgroups

```c
int getgroups(int count, gid_t list[]);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取调用进程的附加组列表（Supplementary Group IDs）。该调用使用 `syscall` 宏（而非 `__syscall`），确保内核返回的错误通过 `__syscall_ret` 转换为 libc 约定。

#### 前置条件

- `count`: 如果 `count == 0`，函数返回附加组的数量而不填充 `list`
- `count`: 如果 `count > 0`，必须大于等于附加组的实际数量
- `list` (当 `count > 0` 时): 指向至少 `count` 个 `gid_t` 元素的非空数组

#### 后置条件

- **Case 1 `count == 0`**
  - 返回附加组的数量
  - `list` 不会被修改

- **Case 2 成功获取（`count >= 实际组数）**
  - `list[0..n-1]` 被填充为进程的附加组 ID
  - 返回实际附加组数量 `n`（`0 <= n <= NGROUPS_MAX`）

- **Case 3 `count` 太小**
  - 返回 -1
  - `errno` 设置为 `EINVAL`
  - `list` 不会被修改

#### 系统算法

```
getgroups(count, list):
  return syscall(SYS_getgroups, count, list)
```

即：
1. 调用 `__syscall(SYS_getgroups, count, list)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

#### 依赖

- `SYS_getgroups` — Linux 内核系统调用编号 (x86_64: 115, aarch64: 158)
- `syscall` — 内部宏，定义在 `src/internal/syscall.h`，等价于 `__syscall_ret(__syscall(...))`
