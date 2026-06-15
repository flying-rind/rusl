# setresgid.c 规约

> musl libc GNU 扩展：设置真实、有效和保存 set-group-ID 系统调用封装。`setresgid` 在 `<unistd.h>` 中声明（需要定义 `_GNU_SOURCE`）。

---

## 依赖图

```
setresgid (Public, GNU 扩展)
  └── __setxid(SYS_setresgid, rgid, egid, sgid)
        └── __synccall(do_setxid, &c)   — 在所有线程上同步执行
        └── __syscall_ret(...)           — 返回值转换
```

---

## 函数规约

### setresgid

```c
#define _GNU_SOURCE
int setresgid(gid_t rgid, gid_t egid, gid_t sgid);
```

[Visibility]: User — `<unistd.h>` GNU 扩展函数，需定义 `_GNU_SOURCE` 后用户程序可直接调用

#### Intent

原子性地同时设置调用进程的真实组 ID、有效组 ID 和保存 set-group-ID。这是 Linux 特有的系统调用 `SYS_setresgid` 的封装，通过 `__setxid` 跨所有线程同步执行。

参数值为 -1 表示对应的 ID 不被修改。与 `setgid` 和 `setregid` 相比，`setresgid` 允许独立控制三个 ID。

#### 前置条件

- `rgid`: 目标真实组 ID（`gid_t` 类型），-1 表示不修改
- `egid`: 目标有效组 ID（`gid_t` 类型），-1 表示不修改
- `sgid`: 目标保存 set-group-ID（`gid_t` 类型），-1 表示不修改
- 需要定义 `_GNU_SOURCE` 特性测试宏以获取此接口声明

#### 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 如果 `rgid != (gid_t)-1`，进程的真实组 ID 已更新
  - 如果 `egid != (gid_t)-1`，进程的有效组 ID 已更新
  - 如果 `sgid != (gid_t)-1`，进程的保存 set-group-ID 已更新
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
setresgid(rgid, egid, sgid):
  return __setxid(SYS_setresgid, rgid, egid, sgid)
```

`__setxid` 算法详见 setxid.c 规约。

#### 依赖

- `SYS_setresgid` — Linux 内核系统调用编号 (x86_64: 119, aarch64: 149)
- `__setxid` — 内部函数，定义在 `src/unistd/setxid.c`，见 setxid.md 规约
