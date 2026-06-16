# sem_timedwait.c 规约

> musl libc 信号量 P 操作（带超时/等待）函数。`sem_wait` 和 `sem_timedwait` 均通过此函数实现。

---

## 依赖图

```
sem_timedwait
  ├─> pthread_testcancel()         (see pthread_cancel.c spec)
  ├─> sem_trywait()                (see sem_trywait.c spec)
  ├─> a_spin()                     (see atomic.h — 自旋等待)
  ├─> a_inc()                      (see atomic.h — 原子递增)
  ├─> a_cas()                      (see atomic.h — 原子比较并交换)
  ├─> pthread_cleanup_push()       (see pthread_cancel.c spec)
  ├─> pthread_cleanup_pop()        (see pthread_cancel.c spec)
  └─> __timedwait_cp()             (see __timedwait.c spec — 可取消 futex 等待)

cleanup (static)
  └─> a_dec()                      (see atomic.h — 原子递减)
```

---

## 函数规约

### 1. cleanup (static)

```c
static void cleanup(void *p);
```

[Visibility]: Internal (不导出) — 文件作用域 static 函数

#### Intent

取消清理函数。当 `sem_timedwait` 中被 `__timedwait_cp` 阻塞的线程被取消时，原子递减等待者计数，保证信号量状态一致性。

#### 前置条件

- `p` 指向 `sem->__val[1]`（即等待者计数器）

#### 后置条件

- `*(int *)p` 原子递减 1（`a_dec(p)`）
- 等待者计数恢复为阻塞前的值

#### 依赖

- `a_dec()` — 原子递减操作

---

### 2. `sem_timedwait`

```c
int sem_timedwait(sem_t *restrict sem, const struct timespec *restrict at);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

递减（锁定）信号量。若信号量值为 0 则阻塞，直到信号量可用或超时。当 `at == NULL` 时等价于 `sem_wait`（无限等待）。

#### 前置条件

- `sem != NULL`，指向有效 `sem_t`
- `at` 可为 `NULL`（无限等待）或指向未来的绝对时间点（`CLOCK_REALTIME`）

#### 后置条件

- Case 1 立即成功（信号量 > 0）：
  - 信号量计数值原子递减 1
  - 返回 `0`
- Case 2 阻塞后成功（被 `sem_post` 唤醒）：
  - 信号量计数值原子递减 1
  - 返回 `0`
- Case 3 超时（`at` 指定的时间到达）：
  - `errno = ETIMEDOUT`
  - 返回 `-1`
- Case 4 线程被取消：
  - 调用 cleanup 函数递减等待者计数
  - 线程不返回

#### 系统算法

```
sem_timedwait(sem, at):
  1. pthread_testcancel()（检查取消点）
  2. 尝试 sem_trywait(sem)：若成功 → 返回 0
  3. 自旋优化：最多 100 次 busy-wait
     若期间信号量变为可用 → 回到步骤 2
  4. while sem_trywait(sem) 失败：
     a. priv = sem->__val[2]
     b. a_inc(sem->__val+1)           // 递增等待者计数
     c. a_cas(sem->__val, 0, 0x80000000)  // 设置等待标记位
     d. pthread_cleanup_push(cleanup, sem->__val+1)  // 注册清理函数
     e. r = __timedwait_cp(sem->__val, 0x80000000, CLOCK_REALTIME, at, priv)
     f. pthread_cleanup_pop(1)        // 弹栈并执行清理函数
     g. 若 r != 0 → errno = r，返回 -1
     h. 循环回到步骤 4 的起始（再次尝试 sem_trywait）
  5. 返回 0（sem_trywait 成功）
```

#### 不变量

- 等待线程被取消时，等待者计数通过 `pthread_cleanup_push/pop` 机制保证正确递减
- 自旋优化（100 次空转）减少短等待场景下的系统调用开销
- `sem->__val[0]` 的 bit 31 在进入等待前被 Cas 为 1（标记有等待者）
- 阻塞等待使用 `CLOCK_REALTIME` 时钟

#### 依赖

- `pthread_testcancel()` — 检查线程取消点
- `sem_trywait()` — 非阻塞信号量递减
- `a_spin()` — 空转自旋（busy-wait 优化）
- `a_inc()` — 原子递增等待者计数
- `a_cas()` — 原子比较并交换（设置等待标记）
- `pthread_cleanup_push()` / `pthread_cleanup_pop()` — 取消清理栈管理
- `__timedwait_cp()` — 可取消的 futex 等待（带超时）
