# setuid.c 规约

> musl libc POSIX 设置用户 ID 系统调用封装。`setuid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
setuid (Public)
  └── __setxid(SYS_setuid, uid, 0, 0)
        └── __synccall(do_setxid, &c)   — 在所有线程上同步执行
        │     ├── __syscall(c->nr, c->id, c->eid, c->sid)
        │     ├── __block_all_sigs(0)    — 阻塞所有信号（失败安全终止路径）
        │     ├── __syscall(SYS_kill, ...) — 发送 SIGKILL
        │     └── __syscall(SYS_getpid)  — 获取当前进程 ID
        └── __syscall_ret(...)           — 内核返回值转换为 libc 约定
```

---

## 函数规约

### setuid

```c
int setuid(uid_t uid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将调用进程的用户 ID 设置为指定值。调用 `__setxid` 跨所有线程同步执行 `SYS_setuid` 系统调用，确保多线程环境下的安全性。

根据 POSIX 规范，`setuid` 的行为取决于调用者是否具有特权（`euid == 0`）：
- 非特权进程只能将有效/真实/保存的 set-user-ID 设置为当前真实用户 ID 或保存的 set-user-ID
- 特权进程（root）设置所有三个 ID（真实、有效、保存）为 `uid`

#### 前置条件

- `uid`: 目标用户 ID（`uid_t` 类型，通常为 `unsigned int`）

#### 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 进程的用户 ID 已更新（具体哪些 ID 被修改取决于内核实现）
  - 返回 0

- **Case 2 权限不足**
  - 返回 -1
  - `errno` 设置为 `EPERM`

- **Case 3 所有线程同步失败**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`
  - 原因：某个线程在 `__synccall` 执行期间因内核级错误失败

- **Case 4 线程间不一致（极端情况）**
  - 若某个线程设置成功而另一线程后续失败，进程将被 `SIGKILL` 强制终止
  - 这是一种安全措施：ID 设置必须原子完成，不一致状态是危险且不可恢复的

#### 系统算法

```
setuid(uid):
  return __setxid(SYS_setuid, uid, 0, 0)
```

`__setxid` 算法详见 setxid.c 规约。

#### 依赖

- `SYS_setuid` — Linux 内核系统调用编号 (x86_64: 105, aarch64: 146)
- `__setxid` — 内部函数，定义在 `src/unistd/setxid.c`，见 setxid.md 规约
- `__synccall` — 内部函数，定义在 `src/internal/__synccall.c`
- `__syscall_ret` — 内核返回值到 libc 错误码转换
