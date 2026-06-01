# aio_suspend 归约

## aio_suspend

```c
int aio_suspend(const struct aiocb *const cbs[], int cnt, const struct timespec *ts);
```

**[Visibility]: Exported (导出)**

### 意图

阻塞调用线程直到 `cbs` 列表中至少一个 AIO 操作完成，或信号递送/超时发生。调用点可被取消。

### 系统算法

1. **取消点检查** — 调用 `pthread_testcancel()` 建立取消点。
2. **参数校验** — 若 `cnt < 0`，置 `errno = EINVAL`，返回 `-1`。
3. **首轮扫描与计数** — 遍历 `cbs[0..cnt-1]`：
   - 跳过 `NULL` 条目；
   - 若任一条目通过 `aio_error` 检测不为 `EINPROGRESS`，立即返回 `0`；
   - 同时统计非空条目数 `nzcnt`，记录最后一个非空 `cb`。
4. **超时绝对时间计算** — 若 `ts != NULL`，通过 `clock_gettime(CLOCK_MONOTONIC, &at)` 获取当前时间并与 `ts` 相加，处理纳秒进位。
5. **主等待循环**：
   - **轮询** — 遍历 `cbs[]`，若任一条目不为 `EINPROGRESS`，返回 `0`。
   - **根据 `nzcnt` 选择等待目标**：
     - `nzcnt == 0`：在栈上创建的 `dummy_fut` 上等待（永不唤醒，仅靠超时/信号触发）。
     - `nzcnt == 1`：在唯一 `cb->__err` 上等待；通过 `a_cas` 将 `EINPROGRESS` 替换为 `EINPROGRESS | 0x80000000` 以通知 worker 有单个等待者。
     - `nzcnt >= 2`：在全局 `__aio_fut` 上等待；首个 waiter 将 `__aio_fut` 从 `0` CAS 为自身 `tid`；CAS 成功或发现已有其他 tid 后，**再次轮询** `cbs[]` 以避免丢失唤醒。
   - **futex 等待** — 调用 `__timedwait_cp(pfut, expect, CLOCK_MONOTONIC, ts ? &at : NULL, 1)`，该函数是取消安全的 futex 等待。
   - **结果处理**：
     - `ETIMEDOUT` → 转为 `EAGAIN`（fallthrough 到错误返回）。
     - `ECANCELED` 或 `EINTR` → 置 `errno = ret`，返回 `-1`。
     - 其他值（如 `0` 表示正常唤醒）→ 回到循环头部重新轮询。

### 前置条件

```
// cnt 为 AIO 控制块指针数组的长度
// cbs 为指向 const struct aiocb* 的指针数组（允许含 NULL）
requires cnt >= 0
requires cbs != NULL || cnt == 0
requires ts == NULL || valid_read(ts, sizeof(struct timespec))
```

### 后置条件

```
// 成功: 至少一个 AIO 操作已完成
ensures result == 0
// 失败: errno 携带错误码
ensures result == -1 ==>
    errno == EINVAL   // cnt < 0 (或实现定义的无效参数)
    || errno == EAGAIN  // 超时（ts 过期）
    || errno == ECANCELED  // 线程被取消
    || errno == EINTR      // 被信号中断
// 无副作用于 cbs 数组中元素指向的 aiocb 结构（读操作）
ensures forall i in [0, cnt). cbs[i] != NULL ==> cbs[i] is unchanged
```

### 不变量

```
// nzcnt 恒为非空 cbs[] 条目计数
invariant nzcnt == count(i in [0, cnt) where cbs[i] != NULL)
// 等待前已轮询过一次，进入 futex 等待时保证没有已完成的操作
invariant before __timedwait_cp: forall i in [0, cnt). cbs[i] == NULL || aio_error(cbs[i]) == EINPROGRESS
```

---

## aio_error

```c
int aio_error(const struct aiocb *cb);
```

**[Visibility]: Exported (导出)**

### 意图

返回 `cb` 关联的 AIO 操作的当前错误状态。`EINPROGRESS` 表示仍在执行中。

### 系统算法

执行 `a_barrier()` 内存屏障后，返回 `cb->__err & 0x7fffffff`（屏蔽最高位，该位由 `aio_suspend` 用作单等待者标记）。

### 前置条件

```
requires cb != NULL
requires valid_read(cb, sizeof(struct aiocb))
```

### 后置条件

```
// 返回值为 0（成功）、EINPROGRESS（进行中）或其他正错误码
ensures result == 0 || result == EINPROGRESS || result > 0
```

---

## a_cas

```c
int a_cas(volatile int *p, int t, int s);
```

**[Visibility]: Internal (不导出)** — 定义于 `src/internal/atomic.h`

### 意图

原子比较并交换：若 `*p == t`，则 `*p = s` 并返回 `t`；否则返回当前 `*p` 值。所有 musl AIO 同步原语的基石。

### 前置条件

```
requires p != NULL
requires valid_write(p, sizeof(int))
```

### 后置条件

```
// 返回操作前的 *p 值
ensures result == old(*p)
// 若返回值为 t，则 *p 已更新为 s
ensures result == t ==> *p == s
```

---

## __aio_fut

```c
extern hidden volatile int __aio_fut;
```

**[Visibility]: Internal (不导出)** — 声明于 `src/internal/aio_impl.h`，定义于 `src/aio/aio.c`

### 意图

全局 futex 变量，供 `aio_suspend`（多等待者）和 `cleanup`（worker 完成时唤醒）之间同步。worker 完成 AIO 后通过 `a_swap(&__aio_fut, 0)` 清除并唤醒等待者。

---

## __timedwait_cp

```c
hidden int __timedwait_cp(volatile int *, int, clockid_t, const struct timespec *, int);
```

**[Visibility]: Internal (不导出)** — 声明于 `src/internal/pthread_impl.h`

### 意图

取消安全的定时 futex 等待。内部调用 futex `FUTEX_WAIT`，但在等待前通过 `__pthread_testcancel` 建立取消点，并处理 `ECANCELED` 唤醒场景。

### 前置条件

```
requires futex_addr != NULL
// expect 为期望的 futex 值
// clk 为时钟 ID (CLOCK_MONOTONIC 等)
// ts 为 NULL（无限等待）或指向绝对超时时间的指针
// priv 为 FUTEX_PRIVATE 标志
```

### 后置条件

```
// 返回 0（正常唤醒）、ETIMEDOUT（超时）、ECANCELED（取消）或 EINTR（信号中断）
ensures result == 0 || result == ETIMEDOUT || result == ECANCELED || result == EINTR
```

---

## __pthread_self

```c
#define __pthread_self() ((pthread_t)(__get_tp() - sizeof(struct __pthread) - TP_OFFSET))
// 或
#define __pthread_self() ((pthread_t)__get_tp())
```

**[Visibility]: Internal (不导出)** — 定义于 `src/internal/pthread_impl.h`

### 意图

获取当前线程的 `pthread_t` 标识符。通过线程指针（TLS）基址推算 `struct __pthread` 地址。