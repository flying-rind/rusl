# pthread_kill.c 规约

> musl libc 的线程定向信号发送函数实现。向指定线程发送信号，在持有目标线程 `killlock` 的情况下确保线程 ID 有效性。

---

## 依赖图

```
pthread_kill
  ├─> __block_all_sigs(&set)      — see internal/pthread_impl.h 或 signal 相关内部函数
  ├─> LOCK(t->killlock)           — see internal/lock.h (自旋锁)
  ├─> __syscall(SYS_tkill, ...)   — see internal/syscall.h
  ├─> UNLOCK(t->killlock)         — see internal/lock.h
  └─> __restore_sigs(&set)        — see internal/pthread_impl.h 或 signal 相关内部函数
```

---

## 对外导出函数规约

### 1. pthread_kill

```c
int pthread_kill(pthread_t t, int sig);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

向指定线程 `t` 发送信号 `sig`。不同于 `kill()`（向进程发送信号），此函数将信号定向到特定线程。

#### 前置条件

- `t` 为有效的 `pthread_t` 值
- `sig` 为有效信号编号，或 0（用于存在性检查）

#### 后置条件

- 操作期间阻塞所有信号（包括内部信号），保证异步取消安全（被 `pthread_cancel` 调用时需要）
- 在持有 `t->killlock` 的情况下检查 `t->tid`：
  - Case 1（`t->tid != 0`，线程有效）：调用 `SYS_tkill(t->tid, sig)` 发送信号，返回系统调用错误码（0 = 成功，正值为 errno）
  - Case 2（`t->tid == 0`，线程已终止或无效）：
    - 若 `sig + 0U >= _NSIG`（无效信号号）：返回 `EINVAL`
    - 否则：返回 0（信号号为 0 时视为成功）
- 操作完成后恢复原信号掩码并释放 `t->killlock`

#### 系统算法

```
pthread_kill(t, sig):
  1. __block_all_sigs(&set)       // 阻塞所有信号, 保证原子性
  2. LOCK(t->killlock)            // 获取目标线程的 kill 锁
  3. if t->tid:                   // 线程仍然有效
       r = -__syscall(SYS_tkill, t->tid, sig)
     else:                        // 线程已终止
       r = (sig + 0U >= _NSIG ? EINVAL : 0)
  4. UNLOCK(t->killlock)
  5. __restore_sigs(&set)         // 恢复原信号掩码
  6. return r
```

#### 不变量

- 操作期间目标线程的 `tid` 受到 `killlock` 保护，不会被修改。这保证了信号不会发往已被复用的线程 ID。

#### 依赖

- `__block_all_sigs(sigset_t *)` — 阻塞所有信号并保存旧掩码
- `__restore_sigs(sigset_t *)` — 恢复先前保存的信号掩码
- `LOCK/UNLOCK` — 自旋锁操作（宏，展开为 `__lock`/`__unlock`，见 `internal/lock.h`）
- `__syscall(SYS_tkill, ...)` — `tkill` 系统调用，向特定线程发送信号
- `_NSIG` — 系统信号总数常量
- `EINVAL` — 错误码（来自 `<errno.h>`）
