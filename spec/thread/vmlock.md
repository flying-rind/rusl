# vmlock.c 规约

> musl libc 内部虚拟内存锁。保护 `fork()` 期间父进程的虚拟内存布局不被其他线程的 `mmap`/`munmap` 操作修改，防止子进程看到不一致的内存映射状态。

---

## 依赖图

```
__vm_wait
  └─> __wait(vmlock, vmlock+1, tmp, 1)   (see __wait.c — futex 等待)

__vm_lock
  └─> a_inc(vmlock)                       (see atomic.h — 原子递增)

__vm_unlock
  ├─> a_fetch_add(vmlock, -1)             (see atomic.h — 原子递减)
  └─> __wake(vmlock, -1, 1)              (see pthread_impl.h — futex 唤醒)
```

---

## 全局变量

### vmlock

```c
static volatile int vmlock[2];
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

双元素数组实现排他锁：
- `vmlock[0]`: 锁计数。0 = 未锁定，正数 = 有线程在虚拟内存操作中
- `vmlock[1]`: 等待者计数。非零表示有线程在等待锁释放

### __vmlock_lockptr

```c
volatile int *const __vmlock_lockptr = vmlock;
```

[Visibility]: Internal (不导出) — 被 `fork_impl.h` 声明为 extern hidden，全局可见

指向 `vmlock` 的常量指针。供 `fork()` 实现和 `__malloc_atfork()` 等函数访问 `vmlock`，用于在 `fork()` 前锁定虚拟内存、`fork()` 后释放。

#### 不变量

- `__vmlock_lockptr` 始终指向 `vmlock[0]`
- 任意时刻，`vmlock[0]` 的值反映正在进行虚拟内存操作的线程数

---

## 函数规约

### 1. __vm_wait

```c
void __vm_wait(void);
```

[Visibility]: Internal (不导出) — 被 fork 相关代码调用，仅 musl 内部使用

#### Intent

等待虚拟内存锁变为可用（即 `vmlock[0] == 0`）。当线程需要进行虚拟内存操作但锁被占用时调用。通过 `__wait` 在 `vmlock` 上进行 futex 等待，并维护等待者计数。

#### 前置条件

- `vmlock[0]` 可能非零（有其他线程持有 VM 锁）
- `vmlock[1]` 为等待者计数（由 `__wait` 内部维护）

#### 后置条件

- 返回时 `vmlock[0] == 0`（锁已释放）
- 等待期间，此线程通过 `__wait` 在 futex 上阻塞

#### 系统算法

```
__vm_wait():
  1. while ((tmp = vmlock[0]) != 0):
  2.   __wait(vmlock, vmlock+1, tmp, 1)    // 等待 vmlock[0] 变为非 tmp
```

#### 依赖

- `__wait()` — futex 等待原语（见 `__wait.c`）

---

### 2. __vm_lock

```c
void __vm_lock(void);
```

[Visibility]: Internal (不导出) — 被 fork 相关代码调用，仅 musl 内部使用

#### Intent

获取虚拟内存锁。原子递增 `vmlock[0]`，阻止其他线程在 `fork()` 期间修改虚拟内存映射。允许多个线程同时持有（引用计数语义），因为递增而非测试并设置。

#### 前置条件

- 无特殊前置条件（原子操作在任何状态下安全）

#### 后置条件

- `vmlock[0]` 原子递增 1
- 若原值非零，表示有其他线程正在进行 VM 操作
- 调用者被视为"持有" VM 锁（但允许多个持有者共存）

#### 系统算法

```
__vm_lock():
  1. a_inc(vmlock)    // 原子递增锁计数
```

#### 依赖

- `a_inc()` — 原子递增（见 `atomic.h`）

---

### 3. __vm_unlock

```c
void __vm_unlock(void);
```

[Visibility]: Internal (不导出) — 被 fork 相关代码调用，仅 musl 内部使用

#### Intent

释放虚拟内存锁。原子递减 `vmlock[0]`。若递减后锁计数为 0（最后一个持有者释放）且有等待者，则唤醒所有等待者。

#### 前置条件

- 调用者此前已调用 `__vm_lock()`（或通过其他方式确保锁计数 > 0）

#### 后置条件

- `vmlock[0]` 原子递减 1
- 若 `a_fetch_add(vmlock, -1) == 1`（递减前值为 1，即最后一个持有者释放）且 `vmlock[1]` 非零（有等待者）：
  - 调用 `__wake(vmlock, -1, 1)` 唤醒所有等待者
- 若无等待者，仅递减计数

#### 系统算法

```
__vm_unlock():
  1. if (a_fetch_add(vmlock, -1) == 1 && vmlock[1]):
  2.   __wake(vmlock, -1, 1)    // 最后一个持有者 + 有等待者 → 全部唤醒
```

#### 依赖

- `a_fetch_add()` — 原子 fetch-and-add（见 `atomic.h`）
- `__wake()` — futex 唤醒内联封装（见 `pthread_impl.h`）
