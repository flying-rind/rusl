# pthread_create.c 规约

> musl libc 线程创建与退出的核心实现。包含线程列表锁（`__tl_lock`/`__tl_unlock`/`__tl_sync`）、线程退出（`__pthread_exit`）、清理处理栈（`__do_cleanup_push`/`__do_cleanup_pop`）、线程启动桩（`start`/`start_c11`）和线程创建（`__pthread_create`）。

---

## 依赖图

```
pthread_create (= __pthread_create)
  ├── __pthread_self
  ├── __acquire_ptc / __release_ptc  (weak, 可能被 pthread_attr_init 覆盖)
  ├── init_file_lock                 (static, 本文件)
  ├── __membarrier_init              (weak)
  ├── __mmap / __mprotect / __munmap  (外部模块: mmap)
  ├── __copy_tls                     (外部模块: tls)
  ├── __block_app_sigs / __restore_sigs (外部模块: thread)
  ├── __tl_lock / __tl_unlock        (static, 本文件)
  ├── __clone                        (外部模块: clone)
  │     └── start / start_c11        (static, 本文件)
  │           └── __pthread_exit     (本文件)
  ├── __ofl_lock / __ofl_unlock      (外部模块: stdio)
  ├── __stdin_used / __stdout_used / __stderr_used (weak)
  └── memcpy / memset                (外部模块: string)

pthread_exit (= __pthread_exit)
  ├── __pthread_self
  ├── __pthread_tsd_run_dtors        (weak)
  ├── __block_app_sigs / __block_all_sigs / __restore_sigs
  ├── a_cas / a_store / a_swap       (外部模块: atomic)
  ├── __vm_wait / __vm_lock / __vm_unlock (外部模块: mmap)
  ├── LOCK / UNLOCK                  (lock.h)
  ├── __tl_lock / __tl_unlock        (static, 本文件)
  ├── __do_orphaned_stdio_locks      (weak)
  ├── __dl_thread_cleanup            (weak)
  ├── __wake / __wait                (外部模块: futex)
  ├── __syscall                      (外部模块: syscall)
  ├── __unmapself                    (外部模块: thread)
  └── exit                           (外部模块: stdlib)
```

---

## 内部类型定义

### struct start_args

```c
struct start_args {
    void *(*start_func)(void *);   // 线程入口函数
    void *start_arg;               // 线程入口参数
    volatile int control;          // 调度控制标志
    unsigned long sig_mask[_NSIG/8/sizeof(long)];  // 信号掩码副本
};
```

[Visibility]: Internal (不导出) — 仅在 `pthread_create.c` 内使用。

#### 字段语义

- `control` 是一个调度控制通信用字段，取值为：
  - `0`: 无需调度设置
  - `1`: 父线程准备执行调度设置（子线程等待）
  - `2`: 子线程确认已等待
  - `3`: 调度设置失败，子线程应退出

---

## 预定义常量和初始化

### Weak 别名定义

```c
weak_alias(dummy_0, __acquire_ptc);          // pthread 创建锁获取
weak_alias(dummy_0, __release_ptc);          // pthread 创建锁释放
weak_alias(dummy_0, __pthread_tsd_run_dtors); // TSD 析构函数运行
weak_alias(dummy_0, __do_orphaned_stdio_locks); // 孤儿 stdio 锁清理
weak_alias(dummy_0, __dl_thread_cleanup);    // 动态链接线程清理
weak_alias(dummy_0, __membarrier_init);      // 内存屏障初始化
```

[Visibility]: Internal (不导出) — 这些 `__` 前缀函数默认指向 `dummy_0`（空函数）。它们可能被其他模块覆盖（如 `pthread_key_create.c` 覆盖 `__pthread_tsd_size`），构成跨模块的依赖注入机制。

---

## 函数规约

### 1. __tl_lock

```c
void __tl_lock(void);
```

[Visibility]: Internal (不导出) — `hidden` 函数，声明于 `pthread_impl.h`。

#### Intent

获取线程列表锁（`__thread_list_lock`）。支持递归获取：如果当前线程已经持有该锁，通过 `tl_lock_count` 计数器实现递归锁定。

#### 前置条件

- 调用者须已阻塞应用层信号（`__block_app_sigs`），以保证 AS-safety
- `__pthread_self()` 返回有效的线程结构体

#### 后置条件

- 线程独占持有线程列表锁：
  - 首次获取：通过 `a_cas` 将 `__thread_list_lock` 从 0 原子设置为当前线程的 `tid`
  - 若锁被其他线程持有：通过 `__wait`（futex）阻塞等待锁释放
  - 递归获取：`tl_lock_count` 递增
- 保证线程列表遍历时的互斥

#### 系统算法

```
__tl_lock():
  1. int tid = __pthread_self()->tid
  2. int val = __thread_list_lock
  3. if val == tid:
       // 递归获取
       tl_lock_count++
       return
  4. // 自旋 + futex 等待
     while (val = a_cas(&__thread_list_lock, 0, tid)):
       __wait(&__thread_list_lock, &tl_lock_waiters, val, 0)
```

---

### 2. __tl_unlock

```c
void __tl_unlock(void);
```

[Visibility]: Internal (不导出) — `hidden` 函数，声明于 `pthread_impl.h`。

#### Intent

释放线程列表锁。支持递归解锁：仅当 `tl_lock_count` 归零后才真正释放锁并唤醒等待者。

#### 前置条件

- 调用者持有线程列表锁（通过 `__tl_lock` 获取）

#### 后置条件

- 递归计数器递减或锁释放：
  - 若 `tl_lock_count > 0`：仅递减计数器，锁保持持有
  - 若 `tl_lock_count == 0`：`__thread_list_lock` 被设置为 0，释放锁
  - 若有等待者（`tl_lock_waiters > 0`），通过 `__wake` 唤醒一个等待者

#### 系统算法

```
__tl_unlock():
  1. if tl_lock_count > 0:
       tl_lock_count--
       return
  2. a_store(&__thread_list_lock, 0)
  3. if tl_lock_waiters:
       __wake(&__thread_list_lock, 1, 0)
```

---

### 3. __tl_sync

```c
void __tl_sync(pthread_t td);
```

[Visibility]: Internal (不导出) — `hidden` 函数，声明于 `pthread_impl.h`。在 `pthread_join.c` 中通过 `weak_alias` 覆盖为实际使用。

#### Intent

等待线程列表锁变为空闲（由任何持有者释放）。用于 `pthread_join` 路径中确保退出线程已经完全从线程列表中移除。

#### 前置条件

- `td` 为被等待的线程

#### 后置条件

- 已知 `__thread_list_lock` 变为 0（空闲）后才返回
- 若有等待者，唤醒一个

#### 系统算法

```
__tl_sync(td):
  1. a_barrier()  // 内存屏障
  2. int val = __thread_list_lock
  3. if val == 0:  return  // 锁已空闲
  4. __wait(&__thread_list_lock, &tl_lock_waiters, val, 0)
  5. if tl_lock_waiters:  __wake(&__thread_list_lock, 1, 0)
```

---

### 4. __do_cleanup_push

```c
void __do_cleanup_push(struct __ptcb *cb);
```

[Visibility]: Internal (不导出) — `hidden` 函数，声明于 `pthread_impl.h`。由 `pthread_cleanup_push` 宏调用。

#### Intent

将清理处理节点 `cb` 推入当前线程的取消清理栈（`cancelbuf` 链表）。当线程被取消或调用 `pthread_exit` 时，这些处理函数将按 LIFO 顺序执行。

#### 前置条件

- `cb != NULL`
- `__pthread_self()` 返回有效的线程结构体

#### 后置条件

- `cb` 被插入 `self->cancelbuf` 链表头部
- `cb->__next` 指向旧链表头

#### 系统算法

```
__do_cleanup_push(cb):
  1. struct pthread *self = __pthread_self()
  2. cb->__next = self->cancelbuf
  3. self->cancelbuf = cb
```

---

### 5. __do_cleanup_pop

```c
void __do_cleanup_pop(struct __ptcb *cb);
```

[Visibility]: Internal (不导出) — `hidden` 函数，声明于 `pthread_impl.h`。由 `pthread_cleanup_pop` 宏调用。

#### Intent

从当前线程的取消清理栈中弹出清理处理节点 `cb`。

#### 前置条件

- `cb != NULL`，且为当前线程清理栈上的节点
- `__pthread_self()` 返回有效的线程结构体

#### 后置条件

- `self->cancelbuf` 被更新为 `cb->__next`（移除 `cb`）

#### 系统算法

```
__do_cleanup_pop(cb):
  1. __pthread_self()->cancelbuf = cb->__next
```

---

### 6. start

```c
static int start(void *p);
```

[Visibility]: Internal (不导出) — `static` 函数，作为新线程的入口桩。

#### Intent

新线程（通过 `clone` 创建）的入口点。处理可选的调度同步、设置信号掩码，然后调用用户提供的线程入口函数，最后调用 `__pthread_exit` 退出。

#### 前置条件

- `p` 指向有效的 `struct start_args`，位于新线程的栈上
- 信号掩码已由父线程配置在 `args->sig_mask` 中

#### 后置条件

- 不返回（调用 `__pthread_exit` 后线程终止）：
  - 线程入口函数已执行
  - 线程通过 `__pthread_exit` 正常退出
- 异常退出：
  - 若 `args->control` 值为 3（调度失败），线程通过 `SYS_exit` 直接退出

#### 系统算法

```
start(p):
  1. struct start_args *args = p
  2. int state = args->control
  3. if state != 0:  // 需要调度设置
       // 等待父线程完成调度设置
       if a_cas(&args->control, 1, 2) == 1:
         __wait(&args->control, 0, 2, 1)  // 等待父线程唤醒
       if args->control != 0:
         // 调度失败 (control == 3)，直接退出
         __syscall(SYS_set_tid_address, &args->control)
         for (;;) __syscall(SYS_exit, 0)
  4. // 恢复信号掩码
     __syscall(SYS_rt_sigprocmask, SIG_SETMASK,
               &args->sig_mask, 0, _NSIG/8)
  5. // 调用用户入口并退出
     __pthread_exit(args->start_func(args->start_arg))
  6. return 0  // 不可达
```

---

### 7. start_c11

```c
static int start_c11(void *p);
```

[Visibility]: Internal (不导出) — `static` 函数，C11 线程的入口桩。

#### Intent

C11 标准线程（`thrd_create`）的入口点。与 `start` 的唯一区别在于对用户入口函数返回值类型的处理：C11 线程入口返回 `int`（`thrd_start_t`），而非 `void *`。

#### 系统算法

```
start_c11(p):
  1. struct start_args *args = p
  2. int (*start)(void*) = (int(*)(void*)) args->start_func
  3. __pthread_exit((void *)(uintptr_t)start(args->start_arg))
  4. return 0  // 不可达
```

---

### 8. init_file_lock

```c
static void init_file_lock(FILE *f);
```

[Visibility]: Internal (不导出) — `static` 函数。

#### Intent

初始化 `FILE` 结构的文件锁。当首次创建线程时（`libc.threaded` 从 0 变为 1），需要为之前单线程模式下未初始化的 `FILE` 锁设置初始值。

#### 前置条件

- `f` 可为 NULL（无操作）

#### 后置条件

- 若 `f != NULL` 且 `f->lock < 0`：`f->lock = 0`（初始化锁为未锁定状态）

---

### 9. __pthread_exit

```c
_Noreturn void __pthread_exit(void *result);
```

[Visibility]: Internal (不导出) — 通过 `weak_alias(__pthread_exit, pthread_exit)` 对外提供 `pthread_exit`。也是 `start`/`start_c11` 的终止路径。

#### Intent

终止当前线程，执行完整的退出清理流程：运行取消清理处理函数、TSDs 析构函数、处理 robust mutex 列表、从线程列表移除自身、对分离线程自动释放资源。

#### 前置条件

- 调用者为退出的线程自身（`__pthread_self()`）
- `result` 为线程返回值（可为 `PTHREAD_CANCELED` 等特殊值）

#### 后置条件

- 线程终止，不返回（`_Noreturn`）：
  - **Joinable 线程**：`detach_state` 被设为 `DT_EXITED`，通过 futex 唤醒 `pthread_join` 等待者，然后循环调用 `SYS_exit`
  - **Detached 线程**：栈映射被 `__unmapself` 释放，线程直接在内核中终止
  - **最后一个线程**（`self->next == self`）：调用 `exit(0)` 终止整个进程

#### 系统算法

```
__pthread_exit(result):
  1. self = __pthread_self()

  // === 阶段 1: 清理准备 ===
  2. self->canceldisable = 1   // 禁止取消
  3. self->cancelasync = 0
  4. self->result = result

  // === 阶段 2: 运行取消清理处理函数 ===
  5. while self->cancelbuf:
       f = self->cancelbuf->__f
       x = self->cancelbuf->__x
       self->cancelbuf = self->cancelbuf->__next
       f(x)

  // === 阶段 3: 运行 TSD 析构函数 ===
  6. __pthread_tsd_run_dtors()

  // === 阶段 4: 阻塞应用信号 ===
  7. __block_app_sigs(&set)

  // === 阶段 5: 原子设置退出状态 ===
  8. state = a_cas(&self->detach_state, DT_JOINABLE, DT_EXITING)

  // === 阶段 6: 分离线程等待 vmlock ===
  9. if state == DT_DETACHED && self->map_base:
       __vm_wait()  // 等待其他线程释放 vmlock

  // === 阶段 7: 获取 killlock ===
  10. LOCK(self->killlock)

  // === 阶段 8: 获取线程列表锁 ===
  11. __tl_lock()

  // === 阶段 9: 最后一个线程检测 ===
  12. if self->next == self:
        // 这是进程中的最后一个线程
        __tl_unlock()
        UNLOCK(self->killlock)
        self->detach_state = state  // 恢复原始状态
        __restore_sigs(&set)
        exit(0)  // 调用 atexit 处理函数然后终止进程

  // === 阶段 10: 清除 TID ===
  13. self->tid = 0
  14. UNLOCK(self->killlock)

  // === 阶段 11: 处理 robust mutex 列表 ===
  15. __vm_lock()
  16. while (rp = self->robust_list.head) && rp != &self->robust_list.head:
        // 解析 pthread_mutex_t 结构体
        m = (void *)((char *)rp - offsetof(pthread_mutex_t, _m_next))
        waiters = m->_m_waiters
        priv = (m->_m_type & 128) ^ 128
        // 从链表移除
        self->robust_list.pending = rp
        self->robust_list.head = *rp
        // 设置 mutex 为 "死锁" 状态并唤醒等待者
        cont = a_swap(&m->_m_lock, 0x40000000)
        self->robust_list.pending = 0
        if cont < 0 || waiters:
          __wake(&m->_m_lock, 1, priv)
  17. __vm_unlock()

  // === 阶段 12: 清理 stdio 和动态链接 ===
  18. __do_orphaned_stdio_locks()
  19. __dl_thread_cleanup()

  // === 阶段 13: 从线程列表移除自身 ===
  20. if !--libc.threads_minus_1:  libc.need_locks = -1
  21. self->next->prev = self->prev
  22. self->prev->next = self->next
  23. self->prev = self->next = self

  // === 阶段 14: 分离线程: 自我销毁 ===
  24. if state == DT_DETACHED && self->map_base:
        __block_all_sigs(&set)
        if self->robust_list.off:
          __syscall(SYS_set_robust_list, 0, 3*sizeof(long))
        __unmapself(self->map_base, self->map_size)

  // === 阶段 15: Joinable 线程: 唤醒 joiner ===
  25. a_store(&self->detach_state, DT_EXITED)
  26. __wake(&self->detach_state, 1, 1)

  27. for (;;) __syscall(SYS_exit, 0)  // 等待内核终止
```

#### 不变量

- **退出原子性**：`detach_state` 的 CAS 操作（步骤 8）是 `pthread_detach` 和 `__pthread_exit` 之间的竞赛仲裁点；CAS 失败者负责资源释放
- **TID 安全性**：步骤 10-14 之间持有 `killlock`，防止其他线程通过 TID 访问正在退出的线程
- **信号安全性**：步骤 7 阻塞应用信号后线程列表锁为 AS-safe
- **最后一个线程处理**：若当前是唯一线程，不进行终止，而是恢复状态并通过 `exit(0)` 调用 atexit

---

### 10. __pthread_create

```c
int __pthread_create(pthread_t *restrict res, const pthread_attr_t *restrict attrp,
                     void *(*entry)(void *), void *restrict arg);
```

[Visibility]: Internal (不导出) — 通过 `weak_alias(__pthread_create, pthread_create)` 对外提供 `pthread_create`。

#### Intent

创建新的 POSIX 线程。负责：首次线程化初始化、属性解析、栈/TLS/TSD 内存分配与映射、`struct pthread` 初始化、通过 `clone` 系统调用创建内核线程、可选的调度属性设置、将新线程插入全局线程列表。

#### 前置条件

- `res != NULL`，指向可写入 `pthread_t` 的内存
- `entry != NULL`，合法的线程入口函数
- `attrp` 可为 NULL（使用默认属性）或指向有效的 `pthread_attr_t`
- `attrp == __ATTRP_C11_THREAD` 表示由 `thrd_create` 调用（C11 线程）
- `libc.can_do_threads` 须为 true（系统支持线程）

#### 后置条件

- Case 1 成功：
  - 新线程被创建并开始执行 `entry(arg)`（或 `start_c11` 包装）
  - `*res` 指向新线程的 `struct pthread`
  - 新线程被插入全局线程列表（双向链表）
  - 返回 0
- Case 2 失败（系统不支持线程）：返回 `ENOSYS`
- Case 3 失败（内存分配/映射失败）：返回 `EAGAIN`
- Case 4 失败（clone 失败）：返回 `EAGAIN`

#### 系统算法

```
__pthread_create(res, attrp, entry, arg):
  1. c11 = (attrp == __ATTRP_C11_THREAD)

  // === 阶段 1: 检查线程支持 ===
  2. if !libc.can_do_threads:  return ENOSYS

  // === 阶段 2: 首次线程化初始化 ===
  3. self = __pthread_self()
  4. if !libc.threaded:
       // 初始化所有已打开 FILE 的锁
       for f = *__ofl_lock(); f != NULL; f = f->next:
         init_file_lock(f)
       __ofl_unlock()
       init_file_lock(__stdin_used)
       init_file_lock(__stdout_used)
       init_file_lock(__stderr_used)
       // 解除阻塞线程信号 (SIGCANCEL 等)
       __syscall(SYS_rt_sigprocmask, SIG_UNBLOCK, SIGPT_SET, 0, _NSIG/8)
       self->tsd = (void **)__pthread_tsd_main
       __membarrier_init()
       libc.threaded = 1

  // === 阶段 3: 解析线程属性 ===
  5. if attrp && !c11:  attr = *attrp  // 复制属性结构体
  6. __acquire_ptc()  // 获取 pthread create 锁
  7. if !attrp || c11:
       // 使用默认属性
       attr._a_stacksize = __default_stacksize
       attr._a_guardsize = __default_guardsize

  // === 阶段 4: 计算栈/TLS/TSD 布局 ===
  8. if attr._a_stackaddr:  // 用户提供了栈地址
       need = libc.tls_size + __pthread_tsd_size
       size = attr._a_stacksize
       stack = (void *)(attr._a_stackaddr & -16)  // 16 字节对齐
       stack_limit = attr._a_stackaddr - size
       if need < size/8 && need < 2048:
         // TLS/TSD 可嵌入用户栈
         tsd = stack - __pthread_tsd_size
         stack = tsd - libc.tls_size
         memset(stack, 0, need)
       else:
         size = ROUND(need)  // 需要单独分配
       guard = 0
     else:  // 使用自动分配的栈
       guard = ROUND(attr._a_guardsize)
       size = guard + ROUND(attr._a_stacksize
                            + libc.tls_size + __pthread_tsd_size)

  // === 阶段 5: 分配栈和 TLS/TSD 内存 ===
  9. if !tsd:  // 尚未获得 TSD 区域
       if guard:
         // 分配 guard 页: 先映射保护页，再映射可读写区域
         map = __mmap(0, size, PROT_NONE, MAP_PRIVATE|MAP_ANON, -1, 0)
         if map == MAP_FAILED:  goto fail
         if __mprotect(map+guard, size-guard, PROT_READ|PROT_WRITE)
            && errno != ENOSYS:
           __munmap(map, size)
           goto fail
       else:
         map = __mmap(0, size, PROT_READ|PROT_WRITE,
                      MAP_PRIVATE|MAP_ANON, -1, 0)
         if map == MAP_FAILED:  goto fail
       tsd = map + size - __pthread_tsd_size
       if !stack:
         stack = tsd - libc.tls_size
         stack_limit = map + guard

  // === 阶段 6: 初始化 struct pthread ===
  10. new = __copy_tls(tsd - libc.tls_size)
  11. new->map_base = map
  12. new->map_size = size
  13. new->stack = stack
  14. new->stack_size = stack - stack_limit
  15. new->guard_size = guard
  16. new->self = new
  17. new->tsd = (void *)tsd
  18. new->locale = &libc.global_locale
  19. new->detach_state = attr._a_detach ? DT_DETACHED : DT_JOINABLE
  20. new->robust_list.head = &new->robust_list.head
  21. new->canary = self->canary
  22. new->sysinfo = self->sysinfo

  // === 阶段 7: 在新线程栈上设置 start_args ===
  23. stack -= (uintptr_t)stack % sizeof(uintptr_t)  // 对齐
  24. stack -= sizeof(struct start_args)
  25. args = (void *)stack
  26. args->start_func = entry
  27. args->start_arg = arg
  28. args->control = attr._a_sched ? 1 : 0

  // === 阶段 8: 设置信号掩码 ===
  29. __block_app_sigs(&set)
  30. memcpy(&args->sig_mask, &set, sizeof args->sig_mask)
  31. // 确保 SIGCANCEL 在新线程中被解除阻塞
      args->sig_mask[(SIGCANCEL-1)/8/sizeof(long)]
        &= ~(1UL<<((SIGCANCEL-1)%(8*sizeof(long))))

  // === 阶段 9: 通过 clone 创建线程 ===
  32. __tl_lock()
  33. if !libc.threads_minus_1++:  libc.need_locks = 1
  34. ret = __clone(
         c11 ? start_c11 : start,  // 线程入口
         stack,                    // 子线程栈顶
         flags,                    // CLONE_VM | CLONE_FS | ... | CLONE_DETACHED
         args,                     // 传递给 start 的参数
         &new->tid,                // CLONE_PARENT_SETTID: 写入 TID
         TP_ADJ(new),              // CLONE_SETTLS: TLS 指针
         &__thread_list_lock       // CLONE_CHILD_CLEARTID: futex 地址
       )

  // === 阶段 10: 处理调度设置 ===
  35. if ret < 0:
        ret = -EAGAIN
      else if attr._a_sched:
        ret = __syscall(SYS_sched_setscheduler,
                        new->tid, attr._a_policy, &attr._a_prio)
        // 通过 control 字段通知子线程调度结果
        if a_swap(&args->control, ret ? 3 : 0) == 2:
          __wake(&args->control, 1, 1)  // 唤醒等待的子线程
        if ret:
          __wait(&args->control, 0, 3, 0)  // 等待子线程退出

  // === 阶段 11: 插入线程列表或清理 ===
  36. if ret >= 0:
        // 成功: 将新线程插入双向链表
        new->next = self->next
        new->prev = self
        new->next->prev = new
        new->prev->next = new
      else:
        if !--libc.threads_minus_1:  libc.need_locks = 0
  37. __tl_unlock()
  38. __restore_sigs(&set)
  39. __release_ptc()

  // === 阶段 12: 失败清理 ===
  40. if ret < 0:
        if map:  __munmap(map, size)
        return -ret  // 返回 EAGAIN
  41. *res = new
  42. return 0

  fail:
    __release_ptc()
    return EAGAIN
```

#### 不变量

- **`libc.need_locks`**：当 `libc.threads_minus_1 > 0` 时设置为 1，表示多线程环境需要锁；当 `libc.threads_minus_1 == 0` 时设置为 -1 或 0，表示可以跳过加锁
- **线程列表**：始终为双向循环链表，`self->next->prev == self && self->prev->next == self`
- **`__thread_list_lock`**：在插入/删除线程时持有，以保证线程列表遍历的一致性
- **clone 标志**：`CLONE_DETACHED` 确保内核不保留父子关系；TID 的 futex 唤醒由 musl 自行管理

---

## 弱别名导出

```c
weak_alias(__pthread_exit,   pthread_exit);    // [Visibility]: User
weak_alias(__pthread_create, pthread_create);  // [Visibility]: User
```

## 依赖汇总

| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `a_cas` / `a_swap` / `a_store` / `a_barrier` | 外部模块 atomic | 原子操作 |
| `__wait` / `__wake` | 外部模块 futex | futex 等待/唤醒 |
| `__syscall` | 外部模块 syscall | 系统调用 |
| `__mmap` / `__mprotect` / `__munmap` | 外部模块 mmap | 内存映射 |
| `__vm_wait` / `__vm_lock` / `__vm_unlock` | 外部模块 mmap | 虚拟内存锁 |
| `__clone` | 外部模块 thread | 线程创建 (clone 系统调用) |
| `__copy_tls` | 外部模块 tls | TLS 数据复制 |
| `__unmapself` | 外部模块 thread | 分离线程自我销毁 |
| `__block_app_sigs` / `__block_all_sigs` / `__restore_sigs` | 外部模块 thread | 信号掩码管理 |
| `__pthread_self` | 外部模块 thread | 获取当前线程结构体 |
| `LOCK` / `UNLOCK` | lock.h (内部) | 自旋锁宏 |
| `__ofl_lock` / `__ofl_unlock` | 外部模块 stdio | 打开文件列表锁 |
| `memcpy` / `memset` | 外部模块 string | 内存操作 |
| `exit` | 外部模块 stdlib | 进程退出 |
| `libc` 全局结构体 | 内部 libc | `can_do_threads`, `threaded`, `need_locks`, `threads_minus_1` 等 |
| `__default_stacksize` / `__default_guardsize` | 外部模块 thread | 默认栈/guard 大小 |
| `__pthread_tsd_size` / `__pthread_tsd_main` | weak / 外部模块 | TSD 区域大小和主线程 TSD 数组 |
