# pthread_once.c 规约

> musl libc 的一次性初始化函数实现。使用原子 CAS + futex 等待/唤醒机制实现多线程安全的单次执行语义，支持初始化函数被取消时自动重置。

---

## 依赖图

```
pthread_once (弱别名 -> __pthread_once)
  ├─> __pthread_once_full(control, init)  (hidden, 同文件)
  │     ├─> a_cas(control, 0, 1)      — see internal/atomic.h
  │     ├─> a_cas(control, 1, 3)      — see internal/atomic.h
  │     ├─> a_swap(control, 2)        — see internal/atomic.h
  │     ├─> pthread_cleanup_push/pop  — see pthread.h (宏)
  │     ├─> undo(control)             — static (同文件)
  │     │     ├─> a_swap(control, 0)  — see internal/atomic.h
  │     │     └─> __wake(control, -1, 1) — see internal/pthread_impl.h
  │     └─> __wait(control, 0, 3, 1)  — see internal/pthread_impl.h
  └─> a_barrier()                     — see internal/atomic.h
```

---

## 内部静态函数规约

### 1. undo (static)

```c
static void undo(void *control);
```

[Visibility]: Internal (不导出) — static 函数，仅在本文件内作为清理回调使用

#### Intent

pthread_once 的取消清理处理函数。当初始化函数 `init()` 被取消时（通过 `pthread_cleanup_push` 注册），将 `once` 控制变量重置回初始状态并唤醒所有等待者。

#### 前置条件

- `control` 为有效的 `pthread_once_t *`

#### 后置条件

- 若 `a_swap(control, 0)` 返回 3（有等待者），则调用 `__wake` 唤醒所有等待线程
- 控制变量重置为 0（初始状态），允许后续调用重新执行初始化

#### 系统算法

```
undo(control):
  1. if a_swap(control, 0) == 3:
       __wake(control, -1, 1)  // 唤醒全部等待者, 私有 futex
```

---

## 内部函数规约

### 2. __pthread_once_full (hidden)

```c
hidden int __pthread_once_full(pthread_once_t *control, void (*init)(void));
```

[Visibility]: Internal (不导出) — `hidden` 可见性，仅库内部使用

#### Intent

执行一次性初始化的核心逻辑。管理 `pthread_once_t` 的状态机（0 -> 1 -> 2），处理多线程并发调用、等待者唤醒、以及初始化函数被取消时的回退。

#### 前置条件

- `control != NULL`，`*control` 为 0（未初始化）或 2（已完成）
- `init != NULL`，为待执行的一次性初始化函数

#### 控制变量状态机

| 值 | 含义 |
|---|------|
| 0 | 未初始化 / 前次被取消后回退 |
| 1 | 某线程正在执行 `init()`，无等待者 |
| 2 | 初始化已完成 |
| 3 | 某线程正在执行 `init()`，有等待者存在 |

#### 后置条件

- Case 1（首个线程，`*control == 0`）：CAS 设为 1，通过 `pthread_cleanup_push` 注册 `undo` 回调，执行 `init()`，成功后以原子 swap 设 `*control = 2`，若旧值为 3 则唤醒等待者。返回 0。
- Case 2（有线程正在初始化，`*control == 1`）：尝试将 1 CAS 为 3（标记等待者存在），通过 `__wait` 在 futex 上睡眠等待，被唤醒后重新循环
- Case 3（有线程正在初始化且有等待者，`*control == 3`）：通过 `__wait` 在 futex 上睡眠等待，被唤醒后重新循环
- Case 4（初始化已完成，`*control == 2`）：立即返回 0
- 若 `init()` 被取消：`pthread_cleanup_pop(0)` 不执行，清理栈调用 `undo()` 将状态重置为 0 并唤醒等待者

#### 系统算法

```
__pthread_once_full(control, init):
  1. loop forever:
       switch a_cas(control, 0, 1):
       case 0:   // 我们是第一个
         pthread_cleanup_push(undo, control)
         init()
         pthread_cleanup_pop(0)     // 不执行 undo
         if a_swap(control, 2) == 3:
           __wake(control, -1, 1)   // 唤醒所有等待者
         return 0
       case 1:   // 有线程在初始化, 无等待者
         a_cas(control, 1, 3)       // 尝试标记有等待者
         // fall through
       case 3:   // 有线程在初始化, 有等待者
         __wait(control, 0, 3, 1)   // futex 等待
         continue                    // 被唤醒后重新检查
       case 2:   // 已完成
         return 0
```

---

### 3. __pthread_once (hidden)

```c
int __pthread_once(pthread_once_t *control, void (*init)(void));
```

[Visibility]: Internal (不导出) — `hidden` 可见性，作为 `pthread_once` 的主实现，见下方弱别名

#### Intent

`pthread_once` 的主实现。先快速检查 `*control == 2`（已完成），利用 volatile 读 + 内存屏障保证 init 副作用对调用者可见，避免在常见路径上进行 futex 系统调用。

#### 前置条件

- `control != NULL`, `init != NULL`

#### 后置条件

- Case 1（`*control == 2`，已完成）：执行 `a_barrier()` 保证 init 副作用可见，返回 0
- Case 2（`*control != 2`）：委托给 `__pthread_once_full()` 处理

#### 系统算法

```
__pthread_once(control, init):
  1. if *(volatile int *)control == 2:
       a_barrier()  // 保证 visibility
       return 0
  2. return __pthread_once_full(control, init)
```

---

## 对外导出函数规约

### 4. pthread_once

```c
int pthread_once(pthread_once_t *control, void (*init)(void));
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)，是 `__pthread_once` 的弱别名

#### Intent

确保 `init` 函数在整个进程生命周期内仅执行一次，无论有多少线程并发调用 `pthread_once` 并传入相同的 `control`。

#### 前置条件

- `control != NULL`，指向静态或全局 `pthread_once_t` 变量，已用 `PTHREAD_ONCE_INIT`（值为 0）静态初始化
- `init != NULL`
- 同一个 `control` 不应在已完成后重新初始化

#### 后置条件

- `init()` 已恰被执行一次（由某个调用线程执行）
- 所有并发/后续调用线程均能看到 `init()` 的完整副作用
- 返回值为 0（成功）
- `*control == 2`
- 若 `init()` 被 pthread 取消，`*control` 被重置为 0，允许后续调用重试

#### 系统算法

```
pthread_once(control, init):
  1. 等同于 __pthread_once(control, init)
```

#### 不变量

- 每个 `pthread_once_t` 控制变量在其生命周期内，关联的 `init` 函数最多成功执行一次
- 状态转换仅在 {0, 1, 2, 3} 之间，且 2 是终结状态

#### 依赖

- `a_cas()` / `a_swap()` — 原子操作（见 `internal/atomic.h`）
- `a_barrier()` — 内存屏障（见 `internal/atomic.h`）
- `__wait()` — futex 等待（见 `internal/pthread_impl.h`）
- `__wake()` — futex 唤醒（见 `internal/pthread_impl.h`）
- `pthread_cleanup_push/pop` — 取消清理栈宏（见 `<pthread.h>`）
