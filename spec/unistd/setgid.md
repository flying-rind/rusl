# setgid.c 规约

> musl libc POSIX 设置组 ID 系统调用封装。`setgid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
setgid (Public)
  └── __setxid(SYS_setgid, gid, 0, 0)
        └── __synccall(do_setxid, &c)   — 在所有线程上同步执行
        └── __syscall_ret(...)           — 返回值转换
```

---

## 函数规约

### setgid

```c
int setgid(gid_t gid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将调用进程的组 ID 设置为指定值。调用 `__setxid` 跨所有线程同步执行 `SYS_setgid` 系统调用。

根据 POSIX 规范：
- 非特权进程只能将有效组 ID 设置为真实组 ID 或保存的 set-group-ID（附加组列表中的组也可以）
- 特权进程（egid == 0）设置真实、有效和保存的 set-group-ID 为 `gid`

#### 前置条件

- `gid`: 目标组 ID（`gid_t` 类型）

#### 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 进程的组 ID 已更新
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
setgid(gid):
  return __setxid(SYS_setgid, gid, 0, 0)
```

`__setxid` 算法详见 setxid.c 规约。

#### 依赖

- `SYS_setgid` — Linux 内核系统调用编号 (x86_64: 106, aarch64: 144)
- `__setxid` — 内部函数，定义在 `src/unistd/setxid.c`，见 setxid.md 规约
