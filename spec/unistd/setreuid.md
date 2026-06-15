# setreuid.c 规约

> musl libc BSD/XOPEN 扩展：设置真实和有效用户 ID 系统调用封装。`setreuid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
setreuid (Public)
  └── __setxid(SYS_setreuid, ruid, euid, 0)
        └── __synccall(do_setxid, &c)   — 在所有线程上同步执行
        └── __syscall_ret(...)           — 返回值转换
```

---

## 函数规约

### setreuid

```c
int setreuid(uid_t ruid, uid_t euid);
```

[Visibility]: User — `<unistd.h>` BSD/XOPEN 扩展函数，用户程序可直接调用

#### Intent

原子性地设置调用进程的真实用户 ID 和有效用户 ID。调用 `__setxid` 跨所有线程同步执行 `SYS_setreuid` 系统调用。

参数值为 -1 表示对应的 ID 不被修改。非特权进程只能将真实用户 ID 设置为当前真实用户 ID 或有效用户 ID，将有效用户 ID 设置为当前真实用户 ID、有效用户 ID 或保存的 set-user-ID。

#### 前置条件

- `ruid`: 目标真实用户 ID（`uid_t` 类型），-1 表示不修改
- `euid`: 目标有效用户 ID（`uid_t` 类型），-1 表示不修改

#### 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 如果 `ruid != (uid_t)-1`，进程的真实用户 ID 已更新
  - 如果 `euid != (uid_t)-1`，进程的有效用户 ID 已更新
  - 对于非特权进程，有效用户 ID 被设置为非 -1 值时，保存的 set-user-ID 也被设置为相同值
  - 返回 0

- **Case 2 权限不足**
  - 返回 -1
  - `errno` 设置为 `EPERM`

- **Case 3 所有线程同步失败**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`

- **Case 4 线程间不一致（极端情况）**
  - 若某个线程设置成功而另一线程后续失败，进程被 `SIGKILL` 强制终止
  - 参见 setxid.md

#### 系统算法

```
setreuid(ruid, euid):
  return __setxid(SYS_setreuid, ruid, euid, 0)
```

`__setxid` 算法详见 setxid.c 规约。

#### 依赖

- `SYS_setreuid` — Linux 内核系统调用编号 (x86_64: 113, aarch64: 145)
- `__setxid` — 内部函数，定义在 `src/unistd/setxid.c`，见 setxid.md 规约
