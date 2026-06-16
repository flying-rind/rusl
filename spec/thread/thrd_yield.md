# thrd_yield.c 规约

> musl libc 的 C11 线程让渡函数实现。直接通过系统调用 `sched_yield` 提示内核主动让出 CPU。

---

## 依赖图

```
thrd_yield
  └─> __syscall(SYS_sched_yield)  — see syscall.h (直接系统调用)
```

---

## 函数规约

### 1. thrd_yield

```c
void thrd_yield(void);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.7)

#### Intent

向操作系统提示：调用线程愿意让出 CPU，允许调度器选择其他线程运行。这是一个优化提示，不保证线程实际被调度出去。直接发出 `sched_yield` 系统调用，避免经过 POSIX 层的 `sched_yield()` 函数包装。

#### 前置条件

- 无

#### 后置条件

- 内核调度器可能将当前线程移到就绪队列末尾，让其他就绪线程运行
- 若没有其他就绪线程，当前线程可能立即继续运行
- 函数无返回值

#### 系统算法

```
thrd_yield():
  1. __syscall(SYS_sched_yield)   // 直接系统调用
  2. return
```

#### 不变量

- 无副作用（仅影响调度优先级，不改变程序语义）

#### 依赖

- `__syscall()` — musl 内部直接系统调用接口（见 `syscall.h`）
- `SYS_sched_yield` — Linux `sched_yield` 系统调用号（见 `syscall.h`）
