# thrd_yield — Rust 接口归约

## 原始 C 接口
```c
void thrd_yield(void);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.7)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn thrd_yield();
```

---

## 意图
向操作系统提示：调用线程愿意让出 CPU，允许调度器选择其他线程运行。这是一个优化提示，不保证线程实际被调度出去。直接发出 `sched_yield` 系统调用，避免经过 POSIX 层的 `sched_yield()` 函数包装。

## 前置条件
- 无

## 后置条件
- 内核调度器可能将当前线程移到就绪队列末尾，让其他就绪线程运行
- 若没有其他就绪线程，当前线程可能立即继续运行
- 函数无返回值

## 不变量
- 无副作用（仅影响调度优先级，不改变程序语义）

## 算法
```rust
extern "C" fn thrd_yield() {
    unsafe {
        // 直接发出 sched_yield 系统调用
        // 内部使用 __syscall(SYS_sched_yield)
        syscall_sched_yield();
    }
}
```

对于 no_std Rust 环境，`sched_yield` 系统调用可直接使用 `libc::sched_yield` 的内联版本，或通过 `core::arch::asm!` 内联汇编发出系统调用。

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 sched_yield 系统调用包装（no_std 兼容，使用内联汇编或 syscall 指令）
pub(crate) unsafe fn syscall_sched_yield();
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __syscall(SYS_sched_yield)           // 依赖1: musl 原始系统调用接口
  或直接使用 Linux sched_yield 系统调用 (无参数)
Predefined Macros/Types:
  SYS_sched_yield (= 架构相关)         // Linux sched_yield 系统调用号

[GUARANTEE]
Exported Interface:
  extern "C" fn thrd_yield();
                                        // 本模块保证对外提供与 C ABI 兼容的 thrd_yield 符号
Internal Interface:
  pub(crate) unsafe fn syscall_sched_yield();
                                        // 内部系统调用包装
