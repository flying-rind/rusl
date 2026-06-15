# setxid — Rust 接口归约

## 原始 C 接口
```c
// 内部结构体
struct ctx {
    int id, eid, sid;
    int nr;
    int ret;
};

// static 回调函数
static void do_setxid(void *p);

// hidden 内部函数
int __setxid(int nr, int id, int eid, int sid);
```

---

## Rust 外部 ABI 接口

> `__setxid` 是 musl 内部 `hidden` 函数，rusl 中需同时提供 `__setxid` 和调用它的公开别名（如 `setuid`/`seteuid` 等均通过它实现）。

```rust
// musl __xx 前缀内部符号 — 与 C ABI 兼容
extern "C" fn __setxid(nr: core::ffi::c_int, id: core::ffi::c_int, eid: core::ffi::c_int, sid: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图

跨所有线程原子性地执行一个 UID/GID 设置系统调用。这是 musl 实现 setuid/setgid 族的核心基础设施，通过 `__synccall` 确保多线程环境下所有线程在同一时刻完成 ID 切换，避免安全漏洞。

## 数据结构

### Ctx — 跨线程上下文

原 C 实现使用 `struct ctx` 通过 `void *` 在线程间传递上下文。Rust 中可以重构为：

```rust
/// 跨线程同步执行的上下文参数
struct XidContext {
    nr: i32,     // 系统调用编号 (SYS_setuid, SYS_setreuid, ...)
    id: i32,     // 真实 ID，-1 表示不修改
    eid: i32,    // 有效 ID，-1 表示不修改
    sid: i32,    // 保存的 set-ID，-1 表示不修改
    ret: i32,    // 执行结果，初始化为 1
}
```

[Visibility]: Internal (不导出) — 仅在 setxid 模块内部使用

---

## 函数规约

### do_setxid — 每线程回调

[Visibility]: Internal (不导出) — 作为 `__synccall` 的回调在进程的每个线程中执行

#### Intent

在每个线程中执行具体的 UID/GID 设置系统调用。设计为 `__synccall` 的回调函数，确保所有线程在一致的时间点执行 ID 切换操作。

该函数实现了关键的安全保证：如果某个线程设置 ID 成功而后续线程失败，说明系统已进入不一致的危险状态，此时唯一安全的做法是使用不可捕获的 `SIGKILL` 强制终止整个进程。

#### 前置条件

- `ctx`: 指向 `XidContext` 的有效引用，由 `__setxid` 初始化后通过 `__synccall` 传入
- `ctx.ret >= 0` 时函数会继续执行；否则（已有线程因信号被中断）提前返回跳过执行

#### 后置条件

- **Case 1 成功执行**
  - `ctx.ret` 被设置为系统调用的返回值
  - 通常情况下 `ctx.ret == 0`

- **Case 2 系统调用返回错误**
  - `ctx.ret` 被设置为内核返回的负错误码
  - 如果之前已有线程成功（`ctx.ret` 初始值 1 表示尚未成功，改为 0 后其他线程检测到 `ctx.ret == 0`），则触发安全终止：
    1. 阻塞所有信号
    2. 调用 `SYS_kill(getpid(), SIGKILL)` 杀死当前进程

- **Case 3 已被信号中断（提前返回）**
  - 当 `ctx.ret < 0` 时（前一线程被信号中断），直接返回不执行任何操作

#### 不变量

- **互斥安全终止**: 如果任何一个线程在另一个线程已成功后失败，进程被不可捕获的 `SIGKILL` 杀死，防止进入 ID 不一致的危险状态

#### 系统算法

```
do_setxid(ctx):
  if ctx.ret < 0: return                     // 已有线程因信号失败，跳过
  ret = __syscall(ctx.nr, ctx.id, ctx.eid, ctx.sid)
  if ret != 0 && ctx.ret == 0:
    // 安全终止：某线程成功后又有一线程失败
    block_all_sigs()
    syscall(SYS_kill, getpid(), SIGKILL)
  ctx.ret = ret
```

Rust 内部实现：

```rust
fn do_setxid(ctx: &mut XidContext) {
    if ctx.ret < 0 {
        return;
    }
    let ret = unsafe { __syscall(ctx.nr, ctx.id, ctx.eid, ctx.sid) };
    if ret != 0 && ctx.ret == 0 {
        // 不一致状态，安全终止
        block_all_signals();
        unsafe { __syscall(SYS_kill, __syscall(SYS_getpid), SIGKILL) };
    }
    ctx.ret = ret;
}
```

---

### __setxid

[Visibility]: Internal (不导出) — musl 中为 `hidden` 可见性，rusl 中作为模块内部函数（`pub(crate)`）同时提供 `extern "C"` 符号

#### Intent

跨所有线程原子性地执行一个 UID/GID 设置系统调用。

#### 前置条件

- `nr`: 有效的系统调用编号（`SYS_setuid`、`SYS_setreuid`、`SYS_setresuid`、`SYS_setgid`、`SYS_setregid`、`SYS_setresgid` 之一）
- `id`: 真实 ID 参数，-1 表示不修改
- `eid`: 有效 ID 参数，-1 表示不修改
- `sid`: 保存的 set-ID 参数，-1 表示不修改

#### 后置条件

- **Case 1 所有线程成功**
  - 返回 0
  - 进程的所有线程的凭据已更新

- **Case 2 某个线程的系统调用失败（EAGAIN）**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`

- **Case 3 系统调用返回非零错误**
  - 返回 -1
  - `errno` 设置为对应的错误码（如 `EPERM`、`EINVAL`）

- **Case 4 线程间不一致（安全终止）**
  - 进程被 `SIGKILL` 杀死，不返回

#### 不变量

- **全有或全无**: 所有线程要么全部成功切换 ID，要么全部失败（或进程被杀死）。不存在部分线程切换、部分未切换的中间状态。

#### 系统算法

```
__setxid(nr, id, eid, sid):
  // ret 初始化为 1（非零），避免首个线程失败时触发安全终止
  ctx = XidContext { nr, id, eid, sid, ret: 1 }
  synccall(do_setxid, &ctx)
  return if ctx.ret > 0 { -EAGAIN } else { ctx.ret }
```

即：
1. 创建上下文结构 `ctx`，`ret` 初始化为 1（非零）
   - 这个初始值 1 是关键设计：它确保 `do_setxid` 中的安全终止逻辑在首个线程失败时不会被触发
2. 调用 `__synccall(do_setxid, &ctx)` 将回调分派到所有线程
3. `__synccall` 返回后，`ctx.ret` 反映了执行结果
4. 如果 `ctx.ret > 0`（初始值 1 未被覆盖，说明首个线程就失败了），转换为 `-EAGAIN`
5. 否则将 `ctx.ret` 转换为 libc 错误码约定

Rust 实现：

```rust
pub(crate) fn __setxid(nr: i32, id: i32, eid: i32, sid: i32) -> i32 {
    let mut ctx = XidContext { nr, id, eid, sid, ret: 1 };
    // 通过 __synccall 在所有线程上同步执行 do_setxid
    synccall(do_setxid, &mut ctx);
    if ctx.ret > 0 {
        -EAGAIN
    } else {
        syscall_ret(ctx.ret)
    }
}
```

---

## 注意事项

- `rusl` 是 `#![no_std]` 实现，需要提供 `__synccall`、`block_all_signals` 等内部基础设施的等价实现
- `__setxid` 的 `hidden` 可见性意味着在 musl 的 ELF 中它不会作为动态符号导出，但 `rusl` 必须同时提供 `__setxid` 内部符号和对应的公开别名函数
- 安全终止路径使用 `SIGKILL`（而不是 `SIGABRT` 或 `abort()`）是因为 `SIGKILL` 不可被捕获、阻塞或忽略，确保进程一定被终止

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __synccall — 内部函数，在所有线程上同步执行回调
  block_all_signals — 内部函数，阻塞所有信号用于安全终止路径
  SYS_kill syscall — Linux 内核系统调用编号 (x86_64: 62, aarch64: 129)
  SYS_getpid syscall — Linux 内核系统调用编号 (x86_64: 39, aarch64: 172)
  SIGKILL — 不可捕获/忽略的信号（值 9）
  __syscall_ret — 内核返回值到 libc 错误码转换
  __syscall — 原始系统调用封装
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)
  synccall 模块 (rusl 内部线程同步封装)
  signal_block 模块 (rusl 内部信号阻塞封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn __setxid(nr: core::ffi::c_int, id: core::ffi::c_int, eid: core::ffi::c_int, sid: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 __setxid 符号
Internal Interface:
  pub(crate) fn __setxid(nr: i32, id: i32, eid: i32, sid: i32) -> i32;
                                 // 模块内部 Rust 实现
