# seteuid.c 规约

> musl libc POSIX 设置有效用户 ID 系统调用封装。`seteuid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
seteuid (Public)
  └── __setxid(SYS_setresuid, -1, euid, -1)
        └── __synccall(do_setxid, &c)   — 在所有线程上同步执行
        │     └── __syscall(SYS_setresuid, -1, euid, -1)
        └── __syscall_ret(...)           — 返回值转换
```

---

## 函数规约

### seteuid

```c
int seteuid(uid_t euid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将调用进程的有效用户 ID 设置为指定值。实现上通过 `__setxid` 调用 `SYS_setresuid(-1, euid, -1)` —— 将真实用户 ID 和保存的 set-user-ID 设为 -1 表示不修改它们。

根据 POSIX 规范：
- 非特权进程只能将有效用户 ID 设置为真实用户 ID 或保存的 set-user-ID
- 特权进程（euid == 0）可以将有效用户 ID 设置为任意值

#### 前置条件

- `euid`: 目标有效用户 ID（`uid_t` 类型）

#### 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 进程的有效用户 ID 已更新
  - 返回 0

- **Case 2 权限不足**
  - 返回 -1
  - `errno` 设置为 `EPERM`

- **Case 3 所有线程同步失败**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`

- **Case 4 线程间不一致（极端情况）**
  - 若某个线程设置成功而另一线程后续失败，进程被 `SIGKILL` 强制终止
  - 参见 setxid.md 中 `do_setxid` 的安全终止逻辑

#### 系统算法

```
seteuid(euid):
  return __setxid(SYS_setresuid, -1, euid, -1)
```

`__setxid` 算法详见 setxid.c 规约。通过将 `ruid` 和 `suid` 参数设为 -1 来保留真实用户 ID 和保存的 set-user-ID 不变。

#### 依赖

- `SYS_setresuid` — Linux 内核系统调用编号 (x86_64: 117, aarch64: 147)
- `__setxid` — 内部函数，定义在 `src/unistd/setxid.c`，见 setxid.md 规约
