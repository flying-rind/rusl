# pthread_once — Rust 接口归约

## 原始 C 接口
```c
// 对外导出（POSIX），是 __pthread_once 的弱别名
int pthread_once(pthread_once_t *control, void (*init)(void));

// 内部（hidden），pthread_once 的主实现
int __pthread_once(pthread_once_t *control, void (*init)(void));

// 内部（hidden），一次性初始化的核心逻辑
hidden int __pthread_once_full(pthread_once_t *control, void (*init)(void));

// 内部 static 清理回调
static void undo(void *control);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// musl 中 pthread_once_t 定义为 int，PTHREAD_ONCE_INIT 为 0
// Rust 中必须同时导出 __pthread_once 和 pthread_once 两个符号

extern "C" fn pthread_once(control: *mut core::ffi::c_int, init: Option<extern "C" fn()>) -> core::ffi::c_int;

// musl 内部实现主函数，也需导出（__ 前缀符号）
extern "C" fn __pthread_once(control: *mut core::ffi::c_int, init: Option<extern "C" fn()>) -> core::ffi::c_int;
```

---

## 意图
确保 `init` 函数在整个进程生命周期内仅执行一次，无论有多少线程并发调用 `pthread_once` 并传入相同的 `control`。使用原子 CAS + futex 等待/唤醒机制实现多线程安全的单次执行语义，支持初始化函数被取消时自动重置。

## 前置条件
- `control` 为非空指针（`!control.is_null()`），指向静态或全局 `pthread_once_t` 变量，已用 `PTHREAD_ONCE_INIT`（值为 0）静态初始化
- `init` 非空（`init.is_some()`）
- 同一个 `control` 不应在已完成后重新初始化

## 控制变量状态机

| 值 | 含义 |
|---|------|
| 0 | 未初始化 / 前次被取消后回退 |
| 1 | 某线程正在执行 `init()`，无等待者 |
| 2 | 初始化已完成 |
| 3 | 某线程正在执行 `init()`，有等待者存在 |

## 后置条件
- `init()` 已恰被执行一次（由某个调用线程执行）
- 所有并发/后续调用线程均能看到 `init()` 的完整副作用
- 返回值为 0（成功）
- `*control == 2`
- 若 `init()` 被 pthread 取消，`*control` 被重置为 0，允许后续调用重试

## 不变量
- 每个 `pthread_once_t` 控制变量在其生命周期内，关联的 `init` 函数最多成功执行一次
- 状态转换仅在 {0, 1, 2, 3} 之间，且 2 是终结状态

## 系统算法

```
__pthread_once(control, init):
  1. 快速路径：volatile 读取 *control == 2，若是则执行内存屏障后返回 0
  2. 慢速路径：委托给 __pthread_once_full(control, init)

__pthread_once_full(control, init):
  1. loop forever:
       switch a_cas(control, 0, 1):
       case 0:   // 我们是第一个执行者
         注册 undo 为取消清理回调
         init()
         弹出清理回调（不执行 undo）
         原子 swap *control = 2
         若旧值为 3（有等待者）则唤醒所有 futex 等待者
         return 0
       case 1:   // 有线程正在初始化，无等待者
         尝试 CAS(1 -> 3) 标记等待者存在
         // fall through
       case 3:   // 有线程正在初始化，已有等待者
         futex 等待 *control == 3
         continue  // 被唤醒后重新检查
       case 2:   // 已完成
         return 0

undo(control):
  1. 原子 swap *control = 0
  2. 若旧值为 3（有等待者），调用 futex wake 唤醒所有等待者
```

## 算法（Rust 实现）

```rust
use core::sync::atomic::{AtomicI32, Ordering, fence};

// __pthread_once — 主入口，快速路径 + 慢速路径
pub extern "C" fn __pthread_once(control: *mut core::ffi::c_int,
    init: Option<extern "C" fn()>) -> core::ffi::c_int {
    // unsafe: volatile 读（Relaxed）检查已完成状态
    let ctrl = unsafe { &*(control as *const AtomicI32) };
    if ctrl.load(Ordering::Relaxed) == 2 {
        fence(Ordering::Acquire);  // 保证 init 副作用可见（替代 a_barrier）
        return 0;
    }
    // 委托给完整逻辑
    __pthread_once_full(control, init)
}

// __pthread_once_full — 核心状态机
fn __pthread_once_full(control: *mut core::ffi::c_int,
    init: Option<extern "C" fn()>) -> core::ffi::c_int {
    let ctrl = unsafe { &*(control as *const AtomicI32) };
    loop {
        match ctrl.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => {
                // 我们是第一个，执行 init
                // 注册 undo 为取消清理（通过 pthread_cleanup_push 等效机制）
                // 注：Rust 中取消清理需通过内部 cleanup 机制实现
                if let Some(f) = init {
                    f();
                }
                // 弹出清理回调（不执行 undo）
                // 原子 swap 完成状态
                let old = ctrl.swap(2, Ordering::Release);
                if old == 3 {
                    // 有等待者，唤醒全部
                    futex_wake(control, -1, 1);
                }
                return 0;
            }
            Err(1) => {
                // 有线程在初始化，无等待者，尝试标记等待者
                let _ = ctrl.compare_exchange(1, 3, Ordering::Relaxed, Ordering::Relaxed);
                // fall through
            }
            Err(3) => {
                // 有等待者，futex 等待
                futex_wait(control, 3, 1);
            }
            Err(2) => {
                // 已完成
                return 0;
            }
            Err(_) => {
                // 意外状态，继续循环
                continue;
            }
        }
    }
}

// undo — 取消清理回调
// 在 Rust 内部安全实现中，可通过 cleanup 机制注册
fn undo(control: *mut core::ffi::c_int) {
    let ctrl = unsafe { &*(control as *const AtomicI32) };
    if ctrl.swap(0, Ordering::Release) == 3 {
        futex_wake(control, -1, 1);
    }
}
```

对 C 调用者：
1. `extern "C" fn pthread_once(control: *mut c_int, init: Option<extern "C" fn()>) -> c_int`
2. 内部调用 `__pthread_once` 的相同逻辑
3. 返回 0 表示成功

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering, fence};

// 安全的 Rust 内部 once 原语（供库内使用）
// 接收 FnOnce 而非 extern "C" 函数指针
pub(crate) fn call_once(control: &AtomicI32, init: impl FnOnce()) {
    // 快速路径
    if control.load(Ordering::Relaxed) == 2 {
        fence(Ordering::Acquire);
        return;
    }
    // 慢速路径：通过内部状态机
    call_once_slow(control, init);
}

// 内部慢速路径实现
fn call_once_slow(control: &AtomicI32, init: impl FnOnce()) {
    // 类似 __pthread_once_full 的逻辑，但使用 Rust 闭包
    // 具体实现依赖 futex 等待/唤醒原语
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  core::sync::atomic::AtomicI32     // 依赖1: 原子 i32，作为 pthread_once_t 的内部表示
  core::sync::atomic::Ordering      // 依赖2: 内存顺序枚举
  core::sync::atomic::fence         // 依赖3: 内存屏障（替代 a_barrier）
  futex_wait / futex_wake           // 依赖4: futex 等待/唤醒原语（内部封装 __syscall(SYS_futex, ...)）
Predefined Macros/Constants:
  PTHREAD_ONCE_INIT (0)            // 依赖5: 初始化值常量
Predefined Structures:
  (none)                            // pthread_once_t 即为 c_int / AtomicI32

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_once(control: *mut core::ffi::c_int, init: Option<extern "C" fn()>) -> core::ffi::c_int;
  extern "C" fn __pthread_once(control: *mut core::ffi::c_int, init: Option<extern "C" fn()>) -> core::ffi::c_int;
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_once 和 __pthread_once 符号
Internal Interface:
  pub(crate) fn call_once(control: &core::sync::atomic::AtomicI32, init: impl FnOnce());
                                    // 安全包装，供 crate 内部使用
