# __wait.c 规约

> musl libc 内部 futex 等待函数。实现带自旋优化的 futex 等待：先在用户态自旋 100 次检查目标值变化，若未变化再进入内核态 futex 等待。

---

## 依赖图

```
__wait
  ├─> a_spin()                            (see atomic.h — 原子自旋/内存屏障)
  ├─> a_inc(waiters)                      (see atomic.h — 原子递增)
  ├─> a_dec(waiters)                      (see atomic.h — 原子递减)
  ├─> __syscall(SYS_futex, ..., FUTEX_WAIT|priv, ...)  (see syscall.h — futex 系统调用)
  └─> __syscall(SYS_futex, ..., FUTEX_WAIT, ...)       (see syscall.h — futex 回退)
```

---

## 函数规约

### 1. __wait

```c
void __wait(volatile int *addr, volatile int *waiters, int val, int priv);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明并定义，仅 musl 内部使用

#### Intent

实现带自旋优化的 futex 等待原语。在进入昂贵的内核态 futex 等待之前，先在用户态自旋最多 100 次检查 `*addr` 是否发生变化。若 `*addr != val`（已被修改），则无需等待直接返回。此策略在锁竞争为中等强度时大幅减少系统调用开销。

可选地通过 `waiters` 参数维护等待者计数，方便调用者实现优化唤醒。

#### 前置条件

- `addr` 非空，指向有效的 futex 字
- `val` 为调用者期望的 `*addr` 当前值
- `waiters` 可为 NULL（不维护等待计数）或指向等待者计数器
- `priv` 为 0（进程共享）或非 0（进程私有）

#### 后置条件

- 返回时 `*addr != val`（值已改变）或已被正常唤醒
- 若 `waiters` 非空：等待开始时 `*waiters` 递增 1，返回前递减 1
- 自旋阶段（最多 100 次）：每次检查 `*addr == val`，若不等则立即返回；否则执行 `a_spin()`（内存屏障/暂停）
- 进入 futex 等待后：循环调用 `SYS_futex FUTEX_WAIT`，若返回 `-ENOSYS` 则回退到不带 `FUTEX_PRIVATE` 标志的调用

#### 系统算法

```
__wait(addr, waiters, val, priv):
  1. spins = 100
  2. if (priv) priv = FUTEX_PRIVATE
  3. // 阶段一：用户态自旋
  4. while (spins-- && (!waiters || !*waiters)):
  5.   if (*addr == val): a_spin()
  6.   else: return                       // 值已变，无需等待
  7. // 阶段二：进入内核态等待
  8. if (waiters) a_inc(waiters)          // 登记等待
  9. while (*addr == val):
 10.   __syscall(SYS_futex, addr, FUTEX_WAIT|priv, val, 0) != -ENOSYS
 11.   || __syscall(SYS_futex, addr, FUTEX_WAIT, val, 0)   // 回退
 12. if (waiters) a_dec(waiters)          // 取消登记
```

#### 不变量

- 仅当 `*addr == val` 时才会阻塞在内核 futex 等待中
- 等待者计数（若启用）在进入 futex 等待前递增、退出后递减，始终保持一致
- 自旋仅在没有已知等待者（`!waiters || !*waiters`）时执行，避免在已有等待者时浪费 CPU

#### 依赖

- `a_spin()` — 原子自旋/内存屏障（见 `atomic.h`）
- `a_inc()` / `a_dec()` — 原子递增/递减（见 `atomic.h`）
- `__syscall()` — 原始系统调用宏（见 `syscall.h`）
- `SYS_futex` / `FUTEX_WAIT` / `FUTEX_PRIVATE` — futex 常量和系统调用号（见 `futex.h`）
