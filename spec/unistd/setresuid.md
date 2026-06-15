# setresuid.c 规约

> musl libc GNU 扩展：设置真实、有效和保存 set-user-ID 系统调用封装。`setresuid` 在 `<unistd.h>` 中声明（需要定义 `_GNU_SOURCE`）。

---

## 依赖图

```
setresuid (Public, GNU 扩展)
  └── __setxid(SYS_setresuid, ruid, euid, suid)
        └── __synccall(do_setxid, &c)   — 在所有线程上同步执行
        └── __syscall_ret(...)           — 返回值转换
```

---

## 函数规约

### setresuid

```c
#define _GNU_SOURCE
int setresuid(uid_t ruid, uid_t euid, uid_t suid);
```

[Visibility]: User — `<unistd.h>` GNU 扩展函数，需定义 `_GNU_SOURCE` 后用户程序可直接调用

#### Intent

原子性地同时设置调用进程的真实用户 ID、有效用户 ID 和保存 set-user-ID。这是 Linux 特有的系统调用 `SYS_setresuid` 的封装，通过 `__setxid` 跨所有线程同步执行。

参数值为 -1 表示对应的 ID 不被修改。与 `setuid` 和 `setreuid` 相比，`setresuid` 允许独立控制三个 ID。

#### 前置条件

- `ruid`: 目标真实用户 ID（`uid_t` 类型），-1 表示不修改
- `euid`: 目标有效用户 ID（`uid_t` 类型），-1 表示不修改
- `suid`: 目标保存 set-user-ID（`uid_t` 类型），-1 表示不修改
- 需要定义 `_GNU_SOURCE` 特性测试宏以获取此接口声明

#### 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 如果 `ruid != (uid_t)-1`，进程的真实用户 ID 已更新
  - 如果 `euid != (uid_t)-1`，进程的有效用户 ID 已更新
  - 如果 `suid != (uid_t)-1`，进程的保存 set-user-ID 已更新
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
setresuid(ruid, euid, suid):
  return __setxid(SYS_setresuid, ruid, euid, suid)
```

`__setxid` 算法详见 setxid.c 规约。

#### 依赖

- `SYS_setresuid` — Linux 内核系统调用编号 (x86_64: 117, aarch64: 147)
- `__setxid` — 内部函数，定义在 `src/unistd/setxid.c`，见 setxid.md 规约
