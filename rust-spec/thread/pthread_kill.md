# pthread_kill — Rust 接口归约

## 原始 C 接口
```c
int pthread_kill(pthread_t t, int sig);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_t 在 musl 中为指向 struct __pthread 的指针
// Rust 中使用 *mut c_void 或自定义不透明指针类型

extern "C" fn pthread_kill(t: pthread_t, sig: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
向指定线程 `t` 发送信号 `sig`。不同于 `kill()`（向进程发送信号），此函数将信号定向到特定线程。在持有目标线程 `killlock` 的情况下确保线程 ID 有效性，防止信号发往已被复用的线程 ID。

## 前置条件
- `t` 为有效的 `pthread_t` 值
- `sig` 为有效信号编号，或 0（用于存在性检查）

## 后置条件
- 操作期间阻塞所有信号（包括内部信号），保证异步取消安全
- 在持有 `t->killlock` 的情况下检查 `t->tid`：
  - Case 1（`t->tid != 0`，线程有效）：调用 `SYS_tkill(t->tid, sig)` 发送信号，返回系统调用错误码（0 = 成功，正值为 errno）
  - Case 2（`t->tid == 0`，线程已终止或无效）：
    - 若 `sig + 0U >= _NSIG`（无效信号号）：返回 `EINVAL`
    - 否则：返回 0（信号号为 0 时视为成功）
- 操作完成后恢复原信号掩码并释放 `t->killlock`

## 不变量
- 操作期间目标线程的 `tid` 受到 `killlock` 保护，不会被修改。这保证了信号不会发往已被复用的线程 ID。

## 算法

```rust
// pthread_kill — 向指定线程发送信号
pub extern "C" fn pthread_kill(t: pthread_t, sig: core::ffi::c_int) -> core::ffi::c_int {
    let mut oldset: Sigset = unsafe { core::mem::zeroed() };

    // 1. 阻塞所有信号，保存旧掩码
    block_all_sigs(&mut oldset);

    // 2. 获取目标线程的 killlock
    lock(&t.killlock);

    // 3. 检查线程有效性并发送信号
    let r = if t.tid != 0 {
        // 线程仍有效，发送定向信号
        let ret = unsafe { __syscall(SYS_tkill, t.tid, sig) };
        if ret < 0 { (-ret) as core::ffi::c_int } else { 0 }
    } else {
        // 线程已终止
        if (sig as core::ffi::c_uint) >= _NSIG as core::ffi::c_uint {
            EINVAL
        } else {
            0
        }
    };

    // 4. 释放锁
    unlock(&t.killlock);

    // 5. 恢复原信号掩码
    restore_sigs(&oldset);

    r
}
```

对 C 调用者：
1. `extern "C" fn pthread_kill(t: pthread_t, sig: c_int) -> c_int`
2. 内部阻塞所有信号 → 获取 `killlock` → 检查 `tid` → `tkill` 系统调用 → 释放锁 → 恢复信号掩码
3. 返回值 0 表示成功，正值为 errno

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
// 使用不透明线程句柄，避免直接暴露 pthread_t 指针

pub(crate) struct ThreadHandle {
    // 内部持有线程引用，确保线程在操作期间有效
    inner: *mut Thread,
}

impl ThreadHandle {
    pub(crate) fn send_signal(&self, sig: Signal) -> Result<(), Errno> {
        // 阻塞信号 → 获取 killlock → 检查 tid → tkill → 释放 → 恢复
        // 返回 Result 表示成功或错误
    }
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  block_all_sigs(sigset_t *)          // 依赖1: 阻塞所有信号并保存旧掩码
  restore_sigs(sigset_t *)            // 依赖2: 恢复先前保存的信号掩码
  lock / unlock                       // 依赖3: 自旋锁操作（保护 killlock）
  __syscall(SYS_tkill, ...)           // 依赖4: tkill 系统调用
Predefined Macros/Constants:
  _NSIG                               // 依赖5: 系统信号总数常量
  EINVAL                              // 依赖6: 错误码
  SYS_tkill                           // 依赖7: 系统调用号
Predefined Structures:
  pthread_t                           // 依赖8: 线程标识符类型（musl 中为指向 struct __pthread 的指针）
  struct __pthread { tid, killlock }  // 依赖9: 线程内部结构

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_kill(t: pthread_t, sig: core::ffi::c_int) -> core::ffi::c_int;
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_kill 符号
Internal Interface:
  pub(crate) fn send_signal_to_thread(handle: &ThreadHandle, sig: Signal) -> Result<(), Errno>;
                                    // 安全包装，供 crate 内部使用
