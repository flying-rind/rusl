# __lock.c 规约

> musl libc 内部自旋锁实现。将锁标志位和拥塞计数器编码到一个 `int` 中，通过原子操作实现高效的线程互斥。用于保护 libc 内部共享数据结构。

---

## 依赖图

```
__lock
  ├─> a_cas(l, 0, INT_MIN + 1)          (see atomic.h — 原子 CAS)
  ├─> a_fetch_add(l, 1)                  (see atomic.h — 原子加法)
  ├─> __futexwait(l, current, 1)         (see pthread_impl.h — futex 等待)
  └─> libc.need_locks                    (see libc.h — 全局锁状态)

__unlock
  ├─> a_fetch_add(l, -(INT_MIN + 1))     (see atomic.h — 原子减法)
  └─> __wake(l, 1, 1)                    (see pthread_impl.h — futex 唤醒)
```

---

## 函数规约

### 1. __lock

```c
void __lock(volatile int *l);
```

[Visibility]: Internal (不导出) — musl 内部锁原语，仅被 libc 内部代码调用

#### Intent

实现一种结合锁标志和拥塞计数的高效自选锁。锁的状态编码在一个 `int` 中：`x == 0` 表示未锁定且无竞争；`x < 0` 表示已锁定，拥塞数 = `x - INT_MIN`；`x > 0` 表示未锁定但有拥塞计数。采用三级策略：快速路径 (CAS)、中等竞争自旋、重度竞争 futex 等待。

#### 前置条件

- `l` 非空，指向有效的 `volatile int` 锁变量
- `libc` 全局结构体已正确初始化
- 调用者不在同一锁的临界区内（非递归锁）

#### 后置条件

- Case 1 单线程模式 (`!libc.need_locks`)：直接返回，不获取锁
- Case 2 快速路径成功 (`a_cas(l, 0, INT_MIN+1)` 返回 0)：锁已获取（`l < 0`），返回
- Case 3 自旋成功：通过自旋重试 CAS 获取锁，返回
- Case 4 重度竞争：进入 futex 等待循环，最终通过 CAS 获取锁，返回
- 所有成功分支均保证：返回时 `l < 0`，调用者持有锁

#### 系统算法

```
__lock(l):
  1. if (!libc.need_locks) return          // 单线程模式，无需锁
  2. current = a_cas(l, 0, INT_MIN + 1)    // 快速路径：尝试直接获取
  3. if (need_locks < 0) libc.need_locks = 0
  4. if (!current) return                  // CAS 成功，已获取锁
  5. for i in 0..9:                        // 中等竞争自旋 (最多10次)
  6.   if (current < 0) current -= INT_MIN + 1  // 去除锁标志
  7.   val = a_cas(l, current, INT_MIN + (current + 1))
  8.   if (val == current) return          // CAS 成功
  9.   current = val
 10. current = a_fetch_add(l, 1) + 1       // 自旋失败，登记拥塞
 11. loop:                                 // 重度竞争 futex 等待循环
 12.   if (current < 0):
 13.     __futexwait(l, current, 1)        // 等待锁释放
 14.     current -= INT_MIN + 1
 15.   val = a_cas(l, current, INT_MIN + current)
 16.   if (val == current) return          // CAS 成功
 17.   current = val
```

#### 不变量

- 锁的状态始终由 `l` 的原子值一致地表达
- 拥塞计数器始终反映等待或持有锁的线程数
- 锁的符号位 (`INT_MIN`) 始终唯一表示"锁定"状态

#### 依赖

- `a_cas()` — 原子 compare-and-swap（见 `atomic.h`）
- `a_fetch_add()` — 原子 fetch-and-add（见 `atomic.h`）
- `__futexwait()` — futex wait 内联封装（见 `pthread_impl.h`）
- `libc.need_locks` — 全局多线程标志（见 `libc.h`）

---

### 2. __unlock

```c
void __unlock(volatile int *l);
```

[Visibility]: Internal (不导出) — musl 内部锁释放原语，仅被 libc 内部代码调用

#### Intent

释放由 `__lock` 获取的锁。检查是否有等待者，如果有则通过 futex wake 唤醒其中一个。是 `__lock` 的配对释放函数。

#### 前置条件

- `l` 非空，指向有效的 `volatile int` 锁变量
- 调用者当前持有该锁（`l[0] < 0`）

#### 后置条件

- 若 `l[0] >= 0`（无竞争），直接返回（无等待者）
- 若 `l[0] < 0`（有锁标志）：
  - 通过 `a_fetch_add(l, -(INT_MIN+1))` 原子地释放锁并减少拥塞计数
  - 若减法后结果不等于 `INT_MIN+1`（意味着还有其他等待者），调用 `__wake(l, 1, 1)` 唤醒一个等待者
- 调用者不再持有锁

#### 系统算法

```
__unlock(l):
  1. if (l[0] < 0):                        // 检查是否有竞争
  2.   if (a_fetch_add(l, -(INT_MIN + 1)) != (INT_MIN + 1)):
  3.     __wake(l, 1, 1)                   // 还有等待者，唤醒一个
```

#### 不变量

- 释放操作是原子的：`a_fetch_add` 保证锁状态的一致转换
- 仅在确实存在等待者时才执行 futex wake（避免不必要的系统调用）

#### 依赖

- `a_fetch_add()` — 原子 fetch-and-add（见 `atomic.h`）
- `__wake()` — futex wake 内联封装（见 `pthread_impl.h`）
