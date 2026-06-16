# pthread_detach — Rust 接口归约

> rusl 将线程置为分离状态。通过原子 CAS 操作将 `detach_state` 从 `DT_JOINABLE` 改为 `DT_DETACHED`；若线程已在退出过程中，退化为阻塞式 join 以完成资源回收。

---

## 原始 C 对外导出接口

```c
// 用户可见符号 (weak_alias)
int pthread_detach(pthread_t t);
int thrd_detach(thrd_t t);   // C11 线程接口

// musl 内部跨模块调用符号 (static 函数通过 weak_alias 导出)
int __pthread_detach(pthread_t t);
```

---

## Rust 外部 ABI 接口

```rust
// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_detach(t: *mut core::ffi::c_void) -> core::ffi::c_int;

// C11 线程接口 — 与 C ABI 兼容
extern "C" fn thrd_detach(t: *mut core::ffi::c_void) -> core::ffi::c_int;

// musl 内部跨模块调用符号 — 与 C ABI 兼容
extern "C" fn __pthread_detach(t: *mut core::ffi::c_void) -> core::ffi::c_int;
```

---

## 意图

将目标线程 `t` 的分离状态从 `DT_JOINABLE` 改为 `DT_DETACHED`。若 CAS 操作失败（线程已进入退出流程或已处于分离/退出状态），则通过 `__pthread_join` 阻塞等待线程退出以完成隐式的资源回收（分离的线程需要自行释放资源）。

---

## 前置条件

- `t` 不为 null，指向有效的 `struct pthread`
- 调用者不能同时是目标线程自身

---

## 后置条件

- Case 1 成功（CAS 成功，`t->detach_state` 从 `DT_JOINABLE` 变为 `DT_DETACHED`）：
  - 目标线程被标记为分离状态
  - 当线程退出时，其资源（栈/映射内存等）将在 `__pthread_exit` 中自动回收
  - 返回 0
- Case 2 失败（`t->detach_state` 不为 `DT_JOINABLE`，线程已在退出/已退出/已分离）：
  - 禁用取消，调用 `__pthread_join(t, null())` 等待线程完成退出
  - 恢复取消状态
  - 始终返回 0

---

## 不变量

- `t->detach_state` 只能通过原子操作修改
- CAS 的三种可能旧值：`DT_JOINABLE`（成功路径）、`DT_EXITING`/`DT_DETACHED`/`DT_EXITED`（join 路径）

---

## 算法

```
__pthread_detach(t):
  1. 尝试原子地将 detach_state 从 DT_JOINABLE 改为 DT_DETACHED
     state = atomic_compare_exchange(&t.detach_state, DT_JOINABLE, DT_DETACHED)
  2. 如果 state != DT_JOINABLE:
       // CAS 失败：线程正在退出或已经分离
       // 禁用取消并 join 以确保资源被回收
       cs = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE)
       __pthread_join(t, null())
       pthread_setcancelstate(cs)
  3. return 0
```

Rust 内部实现可使用 `core::sync::atomic::AtomicI32` 替代 `a_cas`，用 `compare_exchange` 原子操作实现。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32                            // 依赖1: 原子 CAS 操作（替代 a_cas）
  core::ffi::{c_int, c_void}                                // 依赖2: C ABI 基本类型
  __pthread_join          (本模块 pthread_join)             // 依赖3: 阻塞等待线程退出
  __pthread_setcancelstate (外部模块 thread)               // 依赖4: 设置取消状态
  DetachState 枚举定义    (本模块 pthread_create)          // 依赖5: DT_JOINABLE/DT_DETACHED/DT_EXITING/DT_EXITED

Predefined Types:
  struct pthread (pthread_impl.h)                           // 线程结构体定义

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_detach(t: *mut core::ffi::c_void) -> core::ffi::c_int;
  extern "C" fn thrd_detach(t: *mut core::ffi::c_void) -> core::ffi::c_int;
  extern "C" fn __pthread_detach(t: *mut core::ffi::c_void) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的上述符号
