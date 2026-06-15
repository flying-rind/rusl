# setxid.c 规约

> musl libc 内部实现：跨线程同步执行 UID/GID 设置系统调用。`__setxid` 在 `src/internal/libc.h` 中声明为 `hidden`，不对外部用户暴露。所有 `setuid`/`seteuid`/`setgid`/`setegid`/`setreuid`/`setregid`/`setresuid`/`setresgid` 最终都汇集到此函数执行。

---

## 依赖图

```
__setxid (Internal, hidden)
  ├── [struct ctx] — 跨线程上下文传递结构
  ├── do_setxid (static) — 每线程回调函数
  │     ├── __syscall(c->nr, c->id, c->eid, c->sid) — 执行具体系统调用
  │     ├── __block_all_sigs(0)          — 阻塞所有信号（失败安全终止路径）
  │     ├── __syscall(SYS_kill, ...)     — 发送 SIGKILL
  │     └── __syscall(SYS_getpid)        — 获取当前进程 ID
  ├── __synccall(do_setxid, &c)          — 在所有线程上同步执行回调
  └── __syscall_ret(...)                 — 返回值转换为 libc 约定
```

---

## 数据结构

### struct ctx

```c
struct ctx {
    int id, eid, sid;   // 用户/组 ID 参数（真实、有效、保存）
    int nr;              // 系统调用编号 (SYS_setuid, SYS_setreuid, ...)
    int ret;             // 执行结果
};
```

[Visibility]: Internal (不导出) — 仅在 setxid.c 内部使用，通过 `__synccall` 的 `void *` 参数在线程间传递上下文

**字段说明**:
- `id`: 真实 ID（`ruid` 或 `rgid`），-1 表示不修改
- `eid`: 有效 ID（`euid` 或 `egid`），-1 表示不修改
- `sid`: 保存的 set-ID（`suid` 或 `sgid`），-1 表示不修改
- `nr`: 目标系统调用编号，决定实际执行哪个内核系统调用
- `ret`: 系统调用返回值，初始化为 1（非零），用于避免首个线程失败时触发安全终止

---

## 函数规约

### do_setxid (static)

```c
static void do_setxid(void *p);
```

[Visibility]: Internal (不导出) — `static` 函数，作为 `__synccall` 的回调在进程的每个线程中执行

#### Intent

在每个线程中执行具体的 UID/GID 设置系统调用。设计为 `__synccall` 的回调函数，确保所有线程在一致的时间点执行 ID 切换操作。

该函数实现了关键的安全保证：如果某个线程设置 ID 成功而后续线程失败，说明系统已进入不一致的危险状态，此时唯一安全的做法是使用不可捕获的 `SIGKILL` 强制终止整个进程。

#### 前置条件

- `p`: 指向 `struct ctx` 的有效指针，由 `__setxid` 初始化后通过 `__synccall` 传入
- `c->ret >= 0` 时函数会继续执行；否则（已有线程因信号被中断）提前返回跳过执行

#### 后置条件

- **Case 1 成功执行**
  - `c->ret` 被设置为 `__syscall(c->nr, c->id, c->eid, c->sid)` 的返回值
  - 通常情况下 `c->ret == 0`

- **Case 2 系统调用返回错误**
  - `c->ret` 被设置为内核返回的负错误码
  - 如果之前已有线程成功（`c->ret` 初始值 1 表示尚未成功，改为 0 后其他线程检测到 `!c->ret` 为真），则触发安全终止：
    1. 调用 `__block_all_sigs(0)` 阻塞所有信号
    2. 调用 `__syscall(SYS_kill, __syscall(SYS_getpid), SIGKILL)` 杀死当前进程

- **Case 3 已被信号中断（提前返回）**
  - 当 `c->ret < 0` 时（前一线程被信号中断），直接返回不执行任何操作

#### 不变量

- **互斥安全终止**: 如果任何一个线程在另一个线程已成功后失败，进程被不可捕获的 `SIGKILL` 杀死，防止进入 ID 不一致的危险状态

#### 系统算法

```
do_setxid(p):
  ctx = (struct ctx *)p
  if ctx->ret < 0: return                    // 已有线程因信号失败，跳过
  ret = __syscall(ctx->nr, ctx->id, ctx->eid, ctx->sid)
  if ret != 0 && ctx->ret == 0:
    // 安全终止：某线程成功后又有一线程失败
    __block_all_sigs(0)                      // 阻塞所有信号
    __syscall(SYS_kill, __syscall(SYS_getpid), SIGKILL)
  ctx->ret = ret
```

---

### __setxid

```c
int __setxid(int nr, int id, int eid, int sid);
```

[Visibility]: Internal (不导出) — `hidden` 可见性，仅在 `src/internal/libc.h` 内部声明；被所有 setuid/setgid 族函数调用

#### Intent

跨所有线程原子性地执行一个 UID/GID 设置系统调用。这是 musl 实现 setuid/setgid 族的核心基础设施，通过 `__synccall` 确保多线程环境下所有线程在同一时刻完成 ID 切换，避免安全漏洞。

#### 前置条件

- `nr`: 有效的系统调用编号（`SYS_setuid`、`SYS_setreuid`、`SYS_setresuid`、`SYS_setgid`、`SYS_setregid`、`SYS_setresgid` 之一）
- `id`: 真实 ID 参数，-1 表示不修改
- `eid`: 有效 ID 参数，-1 表示不修改
- `sid`: 保存的 set-ID 参数，-1 表示不修改
- 调用者通过 `__synccall` 机制获取的执行上下文必须有效

#### 后置条件

- **Case 1 所有线程成功**
  - 调用者的 `struct ctx.ret` 最终为 0
  - `__syscall_ret(0)` 返回 0，表示成功
  - 进程的所有线程的凭据已更新

- **Case 2 某个线程的系统调用失败（EAGAIN）**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`
  - 原因：首个执行的线程返回了错误（`c.ret > 0`，即初始值 1 被保留或设置为正错误值后经 `__syscall_ret(> 0 ? -EAGAIN : c.ret)` 转换）

- **Case 3 系统调用返回非零错误**
  - 返回 -1
  - `errno` 设置为对应的错误码（如 `EPERM`、`EINVAL`）
  - 此时 `c->ret` 为负错误码，直接传递给 `__syscall_ret`

- **Case 4 线程间不一致（安全终止）**
  - 进程被 `SIGKILL` 杀死，不返回
  - 由 `do_setxid` 内部触发

#### 不变量

- **全有或全无**: 所有线程要么全部成功切换 ID，要么全部失败（或进程被杀死）。不存在部分线程切换、部分未切换的中间状态。

#### 系统算法

```
__setxid(nr, id, eid, sid):
  // ret 初始化为 1（非零），避免首个线程失败时触发安全终止
  c = { .nr = nr, .id = id, .eid = eid, .sid = sid, .ret = 1 }
  __synccall(do_setxid, &c)
  return __syscall_ret(c.ret > 0 ? -EAGAIN : c.ret)
```

即：
1. 创建上下文结构 `ctx`，`ret` 初始化为 1（非零）
   - 这个初始值 1 是关键设计：它确保 `do_setxid` 中的安全终止逻辑在首个线程失败时不会被触发（因为 `ctx->ret == 0` 检查为假）
2. 调用 `__synccall(do_setxid, &c)` 将回调分派到所有线程
   - `__synccall` 会阻塞所有线程、依次执行回调、然后恢复
3. `__synccall` 返回后，`c.ret` 反映了执行结果
4. 如果 `c.ret > 0`（初始值 1 未被覆盖，说明首个线程就失败了），转换为 `-EAGAIN`
5. 否则使用 `__syscall_ret` 将 `c.ret` 转换为 libc 约定

#### 依赖

- `__synccall` — 内部函数，定义在 `src/internal/__synccall.c`，在所有线程上同步执行回调
- `__block_all_sigs` — 内部函数，定义在 `src/signal/block.c`，阻塞所有信号用于安全终止路径
- `SYS_kill` — Linux 内核系统调用编号 (x86_64: 62, aarch64: 129)
- `SYS_getpid` — Linux 内核系统调用编号 (x86_64: 39, aarch64: 172)
- `SIGKILL` — 不可捕获/忽略的信号（值 9）
- `__syscall_ret` — 内核返回值到 libc 错误码转换
- `__syscall` — 原始系统调用宏，定义在 `src/internal/syscall.h`

#### 注意事项

- `rusl` 是 `#![no_std]` 实现，需要提供 `__synccall`、`__block_all_sigs` 等内部基础设施的等价实现
- `__setxid` 的 `hidden` 可见性意味着在 musl 的 ELF 中它不会作为动态符号导出，但 `rusl` 必须同时提供 `__setxid` 内部符号和对应的公开别名函数
- 安全终止路径使用 `SIGKILL`（而不是 `SIGABRT` 或 `abort()`）是因为 `SIGKILL` 不可被捕获、阻塞或忽略，确保进程一定被终止，不会进入未定义的行为状态
