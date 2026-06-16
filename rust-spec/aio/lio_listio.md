# lio_listio — Rust 接口归约

> 对应 C spec: `/home/mangp/桌面/OS/rusl/spec/aio/lio_listio.md`
> 对应头文件: `<aio.h>`

---

## 原始 C 接口

```c
int lio_listio(int mode, struct aiocb *restrict const *restrict cbs, int cnt, struct sigevent *restrict sev);
```

`[Visibility]: User` — 由 `<aio.h>` 声明，符合 POSIX.1-2001 / POSIX.1-2008 标准

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 完全兼容的底层导出函数（safe 接口）
pub extern "C" fn lio_listio(
    mode: core::ffi::c_int,
    cbs: *const *const aiocb,
    cnt: core::ffi::c_int,
    sev: *const sigevent,
) -> core::ffi::c_int;
```

参数和返回值类型与 C ABI 在布局、宽度、调用约定上完全一致，外部 C 代码可透明调用。

---

## 意图

批量发起 I/O 操作列表，支持同步阻塞等待和异步通知两种模式。异步模式下通过内部线程池机制实现完成通知，不依赖内核 AIO 子系统。

## 前置条件

- `mode` 为 `LIO_WAIT` 或 `LIO_NOWAIT`
- `cbs` 为指向 `cnt` 个 `*const aiocb` 指针的数组（若 `cnt == 0` 则为空操作）
- `cnt >= 0`，否则调用失败返回 `-1` 并设 `errno = EINVAL`
- `cbs[i]` 若为非 NULL，其 `aio_lio_opcode` 须为 `LIO_READ`、`LIO_WRITE` 或 `LIO_NOP` 之一
- `sev` 若为非 NULL（仅 `mode == LIO_NOWAIT` 时有意义），其 `sigev_notify` 须为 `SIGEV_NONE`、`SIGEV_SIGNAL` 或 `SIGEV_THREAD` 之一
- `cbs` 数组中的 `aiocb` 对象在操作完成前不得被用户修改

## 后置条件

- **Case 1 (同步模式 `mode == LIO_WAIT`，所有 I/O 成功)**: 阻塞直到所有操作完成，返回 `0`
- **Case 2 (同步模式 `mode == LIO_WAIT`，至少一个 I/O 失败)**: 返回 `-1`，`errno = EIO`
- **Case 3 (异步模式 `mode == LIO_NOWAIT`，`sev == NULL` 或 `sigev_notify == SIGEV_NONE`)**: 立即返回 `0`（I/O 操作已提交至后台线程），无完成通知
- **Case 4 (异步模式 `mode == LIO_NOWAIT`，`sev` 有效且 `sigev_notify != SIGEV_NONE`)**: 立即返回 `0`，创建分离线程（detached thread）后台等待所有 I/O 完成，完成后根据 `sigev_notify` 发送信号或调用回调函数
- **Case 5 (内存分配失败)**: 返回 `-1`，`errno = EAGAIN`
- **Case 6 (任何 `aio_read`/`aio_write` 提交失败)**: 返回 `-1`，`errno = EAGAIN`，已分配的内存被释放
- **Case 7 (线程创建失败)**: 返回 `-1`，`errno = EAGAIN`，已分配的内存被释放
- **Case 8 (参数错误 `cnt < 0`)**: 返回 `-1`，`errno = EINVAL`

## 不变量

- 对每个 `cbs[i]`，其 `aio_lio_opcode` 在提交后保持不变
- 异步模式下，若返回 `0` 且 `sev` 有效，则通知必然在线程完成 I/O 等待后触发
- `LIO_NOP` 类型的操作被跳过，不提交也不等待

## 算法

原 C 实现：
```
lio_listio(mode, cbs, cnt, sev):
  1. 参数校验: cnt < 0 → errno=EINVAL, return -1
  2. 分配 lio_state (仅在 mode==LIO_WAIT 或 (sev && sev->sigev_notify != SIGEV_NONE)):
     - malloc(sizeof *st + cnt * sizeof *cbs)
     - 失败 → errno=EAGAIN, return -1
     - 拷贝 cbs 到 st->cbs (防止用户提前释放)
  3. 遍历提交各 I/O 操作:
     - LIO_READ → aio_read(cbs[i])
     - LIO_WRITE → aio_write(cbs[i])
     - 其他(含 LIO_NOP) → 跳过
     - 提交失败 → free(st), errno=EAGAIN, return -1
  4. 同步路径 (mode==LIO_WAIT):
     - ret = lio_wait(st), free(st), return ret
  5. 异步路径 (需要通知):
     - 初始化 pthread_attr_t, 设置 detached + 最小栈 + 信号掩码
     - pthread_create 创建线程执行 wait_thread(st)
     - 失败 → free(st), 恢复信号掩码, errno=EAGAIN, return -1
     - 成功 → 恢复信号掩码, return 0
  6. return 0
```

Rust 中重新设计为安全风格——内部状态通过 `LioState` 结构体管理，利用 `Option` 和所有权系统消除悬空指针风险：

```rust
pub extern "C" fn lio_listio(
    mode: core::ffi::c_int,
    cbs: *const *const aiocb,
    cnt: core::ffi::c_int,
    sev: *const sigevent,
) -> core::ffi::c_int {
    // 1. 参数校验
    if cnt < 0 {
        unsafe { set_errno(EINVAL); }
        return -1;
    }

    // 2. 判断是否需要分配 LioState（同步等待或异步通知时）
    let need_alloc = mode == LIO_WAIT
        || (!sev.is_null() && unsafe { (*sev).sigev_notify != SIGEV_NONE });

    let mut st: Option<LioState> = if need_alloc {
        match LioState::new(cbs, cnt as usize, sev) {
            Some(st) => Some(st),  // LioState 拥有 cbs 副本的所有权
            None => {
                unsafe { set_errno(EAGAIN); }
                return -1;
            }
        }
    } else {
        None
    };

    // 3. 遍历提交各 I/O 操作（在分配 st 之后，确保失败时能正确释放）
    for i in 0..cnt as usize {
        let cb = unsafe { *cbs.add(i) };
        if cb.is_null() { continue; }
        let opcode = unsafe { (*cb).aio_lio_opcode };
        let ret = match opcode {
            LIO_READ => unsafe { aio_read(cb as *mut aiocb) },
            LIO_WRITE => unsafe { aio_write(cb as *mut aiocb) },
            _ => 0, // LIO_NOP 或其他，跳过
        };
        if ret != 0 {
            // st 的 Drop 自动释放内存
            unsafe { set_errno(EAGAIN); }
            return -1;
        }
    }

    // 4. 同步等待路径
    if mode == LIO_WAIT {
        // SAFETY: st 必定为 Some，因为 mode == LIO_WAIT 时 need_alloc 为 true
        let ret = unsafe { lio_wait(&mut st.as_mut().unwrap_unchecked()) };
        // st 离开作用域时自动 drop，释放内存
        return ret;
    }

    // 5. 异步通知路径
    if let Some(st) = st {
        // SAFETY: st 所有权转移给工作线程，线程负责释放
        return unsafe { spawn_wait_thread(st) };
    }

    // 6. 异步无通知路径（st == None，sev == NULL 或 SIGEV_NONE）
    0
}
```

---

## 内部数据结构

### struct LioState

```rust
/// 封装异步 lio_listio 操作的状态信息，在线程间传递。
///
/// 使用单一堆分配存储结构体及其 cbs 柔性数组，
/// 通过 RAII 模式（Drop trait）确保内存安全释放。
struct LioState {
    sev: Option<NonNull<sigevent>>,
    cnt: usize,
    // cbs 柔性数组紧随结构体分配，通过 raw pointer + 辅助方法以切片形式访问
}
```

`[Visibility]: Internal (不导出)` — 模块私有

**字段说明**:

| 字段 | 类型 | 含义 |
|------|------|------|
| `sev` | `Option<NonNull<sigevent>>` | 异步完成通知配置；`None` 表示不通知 |
| `cnt` | `usize` | AIO 控制块列表中的元素个数 |

cbs 数组（`*const aiocb` 的柔性数组）紧随 `LioState` 在堆上分配，不直接存储为 Rust 字段，而是通过布局计算和辅助方法安全访问。

**Invariant**:
- `cnt` 由 `usize` 类型天然保证非负
- `sev` 为 `None` 或指向有效的 `sigevent`
- 结构体及 cbs 柔性数组通过 `alloc::alloc::alloc` 以单一 `Layout` 分配，通过 `Layout::new::<Self>().extend(Layout::array::<*const aiocb>(cnt))` 计算总大小
- 内存释放由 `Drop` trait 统一管理，不依赖手动 `free`

**安全设计要点**（相比原始 C 设计的改进）:

1. **单一分配模式保留**: 使用 Rust 的 `alloc::alloc::alloc` / `alloc::alloc::dealloc` 配合 `Layout`，保持与 C 柔性数组成员相同的内存紧凑性，同时通过类型系统约束正确的布局计算。
2. **类型安全的访问**: cbs 数组通过 `fn cbs(&self) -> &[*const aiocb]` 方法以切片形式访问，利用 `core::slice::from_raw_parts` 在 unsafe 边界内完成指针运算，调用方获得安全的切片引用。
3. **RAII 内存管理**: 实现 `Drop` trait，在 `LioState` 离开作用域时（包括错误路径的提前返回）自动调用 `dealloc` 释放内存，消除内存泄漏风险。
4. **所有权语义**: `LioState` 不支持 `Clone`，通过所有权转移（move）将内存责任在线程间传递，接收方拥有唯一的所有权。

```rust
impl LioState {
    /// 分配并初始化 LioState 及其 cbs 柔性数组。
    ///
    /// 从用户提供的 cbs 数组拷贝指针副本到内部数组（防止用户提前释放）。
    /// 返回 None 表示内存分配失败。
    fn new(cbs: *const *const aiocb, cnt: usize, sev: *const sigevent) -> Option<Self>;

    /// 以不可变切片引用形式访问 cbs 数组。
    fn cbs(&self) -> &[*const aiocb];

    /// 以可变切片引用形式访问 cbs 数组。
    fn cbs_mut(&mut self) -> &mut [*const aiocb];
}

impl Drop for LioState {
    fn drop(&mut self) {
        // 使用 Layout 计算原始分配大小，调用 alloc::alloc::dealloc 释放
    }
}
```

---

## 内部辅助函数

### lio_wait

```rust
/// 阻塞等待 LioState 中所有异步 I/O 操作完成。
///
/// 遍历 cbs 列表，对每个仍在进行中的操作调用 aio_suspend 阻塞等待。
/// 只要有一个操作最终返回错误码，整体返回失败。
///
/// # Safety
///
/// 调用方须确保 `st` 指向已初始化的 LioState，且其 cbs 数组中的
/// 非 NULL 条目均指向已提交的 aiocb。
unsafe fn lio_wait(st: &mut LioState) -> core::ffi::c_int;
```

`[Visibility]: Internal (不导出)` — 模块私有

/* Hoare-style Specification */

**Pre-condition**:
- `st.cnt >= 0`（由 `usize` 保证）
- `st.cbs()` 中每个非 NULL 元素指向一个已提交的 `aiocb`

**Post-condition**:
- Case 1 (所有 I/O 操作成功完成): 返回 `0`，`errno` 不变
- Case 2 (至少一个 I/O 操作失败): 返回 `-1`，`errno = EIO`
- Case 3 (`aio_suspend` 被信号中断或失败): 返回 `-1`，`errno` 由 `aio_suspend` 设置

**Intent**: 阻塞等待 `st.cbs` 列表中的所有异步 I/O 操作完成，收集各操作是否出错。只要有一个操作出错，整体返回失败。

**System Algorithm**:
1. 初始化 `got_err = false`。
2. 进入无限循环：
   a. 遍历 `i` 从 0 到 `cnt-1`：
      - 若 `cbs[i]` 为 NULL（已处理完毕），跳过。
      - 调用 `aio_error(cbs[i])` 获取操作状态。
      - 若返回 `EINPROGRESS`（仍在进行中），跳出内层循环（需要继续等待）。
      - 若返回非零错误码，置 `got_err = true`。
      - 将 `cbs[i]` 置为 NULL（标记已处理）。
   b. 若 `i == cnt`（所有项已处理完毕）：
      - 若 `got_err`，置 `errno = EIO` 并返回 `-1`。
      - 否则返回 `0`。
   c. 否则，调用 `aio_suspend(cbs.as_ptr() as *const _, cnt, null())` 阻塞等待。
      - 若 `aio_suspend` 返回 `-1`，直接返回 `-1`（将 `errno` 向上传播）。

**Rust 设计改进**:
- 参数从裸指针 `*mut LioState` 改为 `&mut LioState`，借用检查确保调用方持有有效引用
- 通过 `st.cbs_mut()` 安全地修改 cbs 数组中的条目（置 NULL）
- 内部 `unsafe` 仅限：调用 `aio_error`、`aio_suspend` 等外部 FFI 函数

**依赖**:
- `aio_error()` — 本模块（aio），定义于 `aio.rs`
- `aio_suspend()` — 本模块（aio），定义于 `aio_suspend.rs`
- 常量: `EIO`, `EINPROGRESS` — `rusl-errno`

---

### notify_signal

```rust
/// 向当前进程投递一个异步 I/O 完成信号。
///
/// 通过 SYS_rt_sigqueueinfo 系统调用排队发送实时信号，
/// si_code 设为 SI_ASYNCIO，携带预设的 sigev_value。
///
/// # Safety
///
/// 调用方须确保 `sev.sigev_signo` 为有效的实时信号编号。
unsafe fn notify_signal(sev: &sigevent);
```

`[Visibility]: Internal (不导出)` — 模块私有

/* Hoare-style Specification */

**Pre-condition**:
- `sev.sigev_signo` 为有效的实时信号编号
- `sev.sigev_value` 为有效的 `sigval` 值

**Post-condition**:
- 通过 `SYS_rt_sigqueueinfo` 系统调用向当前进程发送实时信号
- `siginfo_t` 的 `si_code` 设置为 `SI_ASYNCIO`，表示异步 I/O 完成
- 调用方线程/进程不会阻塞（信号排队而非等待递送）
- 函数无返回值

**Intent**: 向进程自身投递一个异步 I/O 完成信号，携带预设的 `sigev_value`，供用户注册的信号处理器使用。

**System Algorithm**:
1. 构造 `siginfo_t si`，填充：
   - `si_signo = sev.sigev_signo`（信号编号）
   - `si_value = sev.sigev_value`（随信号传递的值）
   - `si_code = SI_ASYNCIO`（标记为异步 I/O 完成）
   - `si_pid = getpid()`（发送进程 PID）
   - `si_uid = getuid()`（发送进程 UID）
2. 调用 `syscall!(SYS_rt_sigqueueinfo, si.si_pid, si.si_signo, &si)` 将信号入队。

**Rust 设计改进**:
- 参数从裸指针 `*const sigevent` 改为引用 `&sigevent`，消除空指针检查负担
- 使用 `syscall!` 宏封装系统调用，与其他模块统一

**依赖**:
- `getpid()`, `getuid()` — 外部模块 `rusl-unistd`
- `syscall!` 宏 / `SYS_rt_sigqueueinfo` — 内部模块 `rusl-syscall`
- `siginfo_t`, `SI_ASYNCIO` — 外部模块 `rusl-signal`

---

### wait_thread

```rust
/// 异步 lio_listio 的工作线程入口。
///
/// 接收 LioState 的所有权，阻塞等待所有 I/O 完成后，
/// 根据 sigev_notify 配置执行信号通知或回调调用。
///
/// 必须作为 pthread_create 的线程入口被调用，
/// 线程以分离状态运行（PTHREAD_CREATE_DETACHED），退出后自动回收。
extern "C" fn wait_thread(p: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
```

`[Visibility]: Internal (不导出)` — 模块私有

/* Hoare-style Specification */

**Pre-condition**:
- `p` 指向由 `LioState::new()` 动态分配的 `LioState`，所有权唯一且有效
- `sev`（从 `LioState` 中提取）为 `None` 或指向有效的 `sigevent`，且 `sigev_notify` 为 `SIGEV_SIGNAL` 或 `SIGEV_THREAD`
- 本函数作为 `pthread_create` 的线程入口被调用

**Post-condition**:
- 调用 `lio_wait(st)` 阻塞等待所有 I/O 完成
- `LioState` 的所有权被回收（`Box::from_raw` 后自动 drop），内存释放
- Case 1 (`sev.sigev_notify == SIGEV_SIGNAL`): 调用 `notify_signal(&sev)` 向进程发送完成信号
- Case 2 (`sev.sigev_notify == SIGEV_THREAD`): 调用 `(sev.sigev_notify_function)(sev.sigev_value)` 执行用户回调
- 返回 `null_mut()`
- 线程以分离状态运行，退出后自动回收资源

**Intent**: 作为异步 `lio_listio` 的工作线程入口，等待所有 I/O 完成后，根据 `sigev_notify` 配置执行相应通知——发送信号或调用用户回调函数。

**System Algorithm**:
1. 从参数 `p` 通过 `Box::from_raw(p as *mut LioState)` 获取 `LioState` 所有权。
2. 提取 `sev` 的副本（在 `LioState` 被 drop 前获取，避免 use-after-free）：
   - 若 `sev` 为 `None`，直接返回 `null_mut()`（无通知）。
   - 否则提取 `sev` 指针对应的 `sigevent` 必要字段。
3. 调用 `lio_wait(&mut st)` 阻塞等待所有 I/O 完成。
4. `st` 离开作用域时自动执行 `Drop::drop`，释放堆内存。
5. 根据 `sev.sigev_notify` 分支：
   - `SIGEV_SIGNAL`: 调用 `notify_signal(&sev_copy)`。
   - `SIGEV_THREAD`: 调用 `(sev_copy.sigev_notify_function)(sev_copy.sigev_value)`。
6. 返回 `null_mut()`。

**Rust 设计改进**:
- 使用 `Box::from_raw` 将 `pthread_create` 传入的裸指针安全地转换为 `Box<LioState>`，获得所有权
- `LioState` 的 `Drop` 实现确保即使在线程取消的异常路径下也能正确释放内存
- `sev` 相关字段在 `Box::from_raw` 后、`lio_wait` 前提取，避免 borrow-after-move 和 use-after-free
- 函数签名保持 `extern "C"` 以兼容 `pthread_create` 回调约定，但内部使用模块私有 safe/unsafe 边界管理

**依赖**:
- `lio_wait()` — 本文件内部
- `notify_signal()` — 本文件内部
- `SIGEV_SIGNAL`, `SIGEV_THREAD` — 外部模块 `rusl-signal`

---

## Rust 辅助工具函数（模块内部）

```rust
/// 判断是否需要为异步操作分配 LioState。
///
/// 需要分配的条件:
/// - mode == LIO_WAIT (同步等待需要 LioState 来收集结果)
/// - sev 非 NULL 且 sigev_notify != SIGEV_NONE (异步通知需要线程间状态传递)
#[inline]
fn needs_async_state(mode: core::ffi::c_int, sev: *const sigevent) -> bool {
    mode == LIO_WAIT
        || (!sev.is_null() && unsafe { (*sev).sigev_notify != SIGEV_NONE })
}

/// 为异步通知路径创建分离工作线程。
///
/// 设置线程属性（detached + 最小栈），阻塞所有信号供子线程继承，
/// 调用 pthread_create 创建线程执行 wait_thread。
///
/// 成功时 LioState 的所有权转移给新线程；失败时 LioState 被 drop。
///
/// # Safety
///
/// 调用方须确保 `st` 是有效的 LioState 实例。
unsafe fn spawn_wait_thread(st: LioState) -> core::ffi::c_int {
    // 1. 将 LioState 转为裸指针以传递给 pthread_create
    //    (使用 Box::into_raw 避免提前 drop)
    // 2. 初始化 pthread_attr_t:
    //    - SIGEV_THREAD: 使用用户提供的属性或默认属性
    //    - 其他: 设置栈大小为 PAGE_SIZE，保护页大小为 0
    // 3. 设置 PTHREAD_CREATE_DETACHED
    // 4. 阻塞所有信号 (sigfillset + pthread_sigmask)
    // 5. pthread_create
    //    - 成功: 恢复信号掩码，返回 0
    //    - 失败: Box::from_raw 回收 LioState 所有权（自动 drop），
    //            恢复信号掩码，set_errno(EAGAIN)，返回 -1
}
```

---

/* Rely */
[RELY]
Predefined Structures/Types:
  aiocb                                        // 依赖1: AIO 控制块类型，由 rusl-aio 模块提供 (#[repr(C)])
  sigevent                                     // 依赖2: 信号事件结构体，由 rusl-signal 模块提供 (#[repr(C)])
  siginfo_t                                    // 依赖3: 信号信息类型，由 rusl-signal 模块提供 (#[repr(C)])
  sigset_t                                     // 依赖4: 信号集类型，由 rusl-signal 模块提供
  pthread_attr_t                               // 依赖5: 线程属性类型，由 rusl-pthread 模块提供
  pthread_t                                    // 依赖6: 线程 ID 类型，由 rusl-pthread 模块提供
  timespec                                     // 依赖7: 时间规格类型，由 rusl-time 模块提供 (aio_suspend 间接使用)
Predefined Functions:
  aio_read(aiocb: *mut aiocb) -> c_int         // 依赖8: 本模块，定义于 aio.rs
  aio_write(aiocb: *mut aiocb) -> c_int        // 依赖9: 本模块，定义于 aio.rs
  aio_error(aiocb: *const aiocb) -> c_int      // 依赖10: 本模块，定义于 aio.rs
  aio_suspend(cbs: *const *const aiocb, cnt: c_int, timeout: *const timespec) -> c_int
                                                // 依赖11: 本模块，定义于 aio_suspend.rs
  getpid() -> pid_t                            // 依赖12: 外部模块 rusl-unistd
  getuid() -> uid_t                            // 依赖13: 外部模块 rusl-unistd
  pthread_create(td: *mut pthread_t, attr: *const pthread_attr_t, start: extern "C" fn(*mut c_void) -> *mut c_void, arg: *mut c_void) -> c_int
                                                // 依赖14: 外部模块 rusl-pthread
  pthread_attr_init(attr: *mut pthread_attr_t) -> c_int
                                                // 依赖15: 外部模块 rusl-pthread
  pthread_attr_setstacksize(attr: *mut pthread_attr_t, size: size_t) -> c_int
                                                // 依赖16: 外部模块 rusl-pthread
  pthread_attr_setguardsize(attr: *mut pthread_attr_t, size: size_t) -> c_int
                                                // 依赖17: 外部模块 rusl-pthread
  pthread_attr_setdetachstate(attr: *mut pthread_attr_t, state: c_int) -> c_int
                                                // 依赖18: 外部模块 rusl-pthread
  pthread_sigmask(how: c_int, set: *const sigset_t, old: *mut sigset_t) -> c_int
                                                // 依赖19: 外部模块 rusl-signal
  sigfillset(set: *mut sigset_t) -> c_int      // 依赖20: 外部模块 rusl-signal
Predefined Macros/Crates:
  alloc::alloc::{alloc, dealloc}               // 依赖21: no_std 内存分配（alloc crate）
  alloc::alloc::Layout                         // 依赖22: 内存布局描述，用于柔性数组分配计算
  core::ptr::NonNull                           // 依赖23: 非空指针抽象，用于 sev 字段
  core::ffi::c_void, c_int                     // 依赖24: FFI 兼容类型
  syscall! 宏                                  // 依赖25: 系统调用入口（rusl-syscall 提供）
Predefined Constants:
  LIO_WAIT, LIO_NOWAIT, LIO_READ, LIO_WRITE, LIO_NOP
                                                // 依赖26: 操作模式常量，由 rusl-aio 模块提供
  SIGEV_NONE, SIGEV_SIGNAL, SIGEV_THREAD        // 依赖27: 通知模式常量，由 rusl-signal 模块提供
  SI_ASYNCIO                                    // 依赖28: 异步 I/O 信号码，由 rusl-signal 模块提供
  SYS_rt_sigqueueinfo                           // 依赖29: 系统调用编号，由 rusl-syscall 模块提供（架构相关）
  SIG_BLOCK, SIG_SETMASK                        // 依赖30: 信号掩码操作常量，由 rusl-signal 模块提供
  PTHREAD_CREATE_DETACHED                       // 依赖31: 分离线程属性常量，由 rusl-pthread 模块提供
  PAGE_SIZE                                     // 依赖32: 页大小常量，由 rusl-internal 模块提供
  EINVAL, EAGAIN, EIO, EINPROGRESS              // 依赖33: errno 常量，由 rusl-errno 模块提供
  set_errno                                     // 依赖34: errno 设置函数，由 rusl-errno 模块提供

[GUARANTEE]
Exported Interface:
  pub extern "C" fn lio_listio(
      mode: core::ffi::c_int,
      cbs: *const *const aiocb,
      cnt: core::ffi::c_int,
      sev: *const sigevent,
  ) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 完全兼容的 lio_listio 符号，
                                   // 满足 POSIX.1-2001/2008 标准要求，
                                   // 支持 LIO_WAIT 同步阻塞和 LIO_NOWAIT 异步通知两种模式

Internal Interface (模块私有，不对外暴露):
  struct LioState;                 // 内部状态结构体，单一堆分配存储结构体 + cbs 柔性数组
  impl LioState::new(...) -> Option<Self>;
                                   // 构造函数：分配并初始化，返回 None 表示 OOM
  impl LioState::cbs(&self) -> &[*const aiocb];
                                   // 以不可变切片引用安全访问 cbs 数组
  impl LioState::cbs_mut(&mut self) -> &mut [*const aiocb];
                                   // 以可变切片引用安全访问 cbs 数组
  impl Drop for LioState;          // RAII 析构：自动释放堆内存
  unsafe fn lio_wait(st: &mut LioState) -> c_int;
                                   // 内部等待函数：阻塞至所有 I/O 完成，收集错误
  unsafe fn notify_signal(sev: &sigevent);
                                   // 内部通知函数：通过 SYS_rt_sigqueueinfo 发送实时信号
  extern "C" fn wait_thread(p: *mut c_void) -> *mut c_void;
                                   // 工作线程入口：pthread_create 回调，通过 Box::from_raw 获取 LioState 所有权
  fn needs_async_state(mode: c_int, sev: *const sigevent) -> bool;
                                   // 辅助判断：是否需要分配 LioState
  unsafe fn spawn_wait_thread(st: LioState) -> c_int;
                                   // 辅助函数：创建分离线程并转移 LioState 所有权
