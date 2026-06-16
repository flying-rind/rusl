# pthread_self — Rust 接口归约

## 原始 C 接口
```c
// 对外导出（POSIX），__attribute__((const)) 纯函数
pthread_t pthread_self(void);

// 对外导出（ISO C11），thrd_current 的弱别名目标
pthread_t thrd_current(void);

// 内部 static 函数，封装 __pthread_self() 调用
static pthread_t __pthread_self_internal(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_t 在 musl 中为指向 struct __pthread 的指针
// Rust 中 pthread_t 定义为不透明指针类型
// 必须同时导出所有符号

extern "C" fn pthread_self() -> pthread_t;

extern "C" fn thrd_current() -> pthread_t;
```

---

## 意图
返回调用线程的线程标识符（`pthread_t`）。无需系统调用，直接通过 TLS 寄存器读取线程指针。在 musl 中，`pthread_t` 即指向 `struct __pthread` 的指针。

## 前置条件
- 调用线程必须已通过 `pthread_create` 创建，或为主线程（主线程的 `pthread_t` 由 libc 初始化时设置）

## 后置条件
- 返回值为调用线程的 `pthread_t`，即指向当前线程 `struct __pthread` 的指针
- 无副作用，不修改任何全局或线程局部状态
- 保证返回值在调用线程的生命周期内有效且唯一
- 声明为 `__attribute__((const))` 纯函数：给定参数（无参）总是返回相同值（对于同一线程）

## 不变量
- 每个线程的 `pthread_t` 在其生命期内不变且唯一

## 算法

```rust
// pthread_t 的类型定义（与 musl ABI 兼容）
// musl 中 pthread_t 为指向 struct __pthread 的指针
pub type pthread_t = *mut Thread;  // Thread = struct __pthread

// __pthread_self_internal — 内部统一入口
// 通过 TLS 读取当前线程指针
fn __pthread_self_internal() -> pthread_t {
    // 读取 TLS 线程指针寄存器（如 x86-64 的 fs 段寄存器）
    // 在 musl 中: (pthread_t)__get_tp() 或
    //            (pthread_t)(__get_tp() - sizeof(struct __pthread) - TP_OFFSET)
    // 取决于 TLS_ABOVE_TP 配置
    __get_tp() as pthread_t
}

// pthread_self — POSIX 对外导出
// 直接读取 TLS 寄存器，无系统调用
pub extern "C" fn pthread_self() -> pthread_t {
    __pthread_self_internal()
}

// thrd_current — C11 对外导出
pub extern "C" fn thrd_current() -> pthread_t {
    __pthread_self_internal()
}
```

对 C 调用者：
1. `extern "C" fn pthread_self() -> pthread_t` 无参数
2. 内部直接读取 TLS 线程指针寄存器
3. 返回当前线程的 `pthread_t` 值

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
// 返回对当前线程 Thread 结构体的引用
pub(crate) fn current_thread() -> &'static Thread {
    unsafe {
        let tp = __get_tp();
        // 根据 TLS 布局计算 Thread 结构体基址
        &*(tp as *const Thread)
    }
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  __get_tp()                              // 依赖1: 架构特定的内联汇编/内置函数，读取 TLS 线程指针寄存器
Predefined Macros/Constants:
  TLS_ABOVE_TP / TP_OFFSET                // 依赖2: TLS 布局配置（编译时常量）
Predefined Structures:
  Thread (struct __pthread)               // 依赖3: 线程内部结构体
  pthread_t                               // 依赖4: 不透明线程标识符类型（musl 中为指针）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_self() -> pthread_t;
  extern "C" fn thrd_current() -> pthread_t;
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_self 和 thrd_current 符号
Internal Interface:
  pub(crate) fn current_thread() -> &'static Thread;
                                    // 安全包装，供 crate 内部使用
