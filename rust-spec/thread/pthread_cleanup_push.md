# pthread_cleanup_push / pthread_cleanup_pop — Rust 接口归约

## 原始 C 接口
```c
// ========== C 宏定义（<pthread.h>）==========

#define pthread_cleanup_push(f, x) do { struct __ptcb __cb; _pthread_cleanup_push(&__cb, f, x);
#define pthread_cleanup_pop(r) _pthread_cleanup_pop(&__cb, (r)); } while(0)

// ========== 底层函数（对外声明）==========

// 压入清理处理函数到取消清理栈
void _pthread_cleanup_push(struct __ptcb *cb, void (*f)(void *), void *x);

// 弹出清理处理函数
void _pthread_cleanup_pop(struct __ptcb *cb, int run);

// ========== 数据结构 ==========

struct __ptcb {
    void (*__f)(void *);   // 清理回调函数指针
    void *__x;             // 回调参数
    struct __ptcb *__next; // 链表下一节点
};

// ========== 内部依赖 ==========

// 内部 static 占位空函数（启动早期充当弱别名）
static void dummy(struct __ptcb *cb);

// hidden 函数（其他 pthread 模块实现）
void __do_cleanup_push(struct __ptcb *cb);
void __do_cleanup_pop(struct __ptcb *cb);
```

---

## Rust 外部 ABI 接口

```rust
// ========== 数据结构（repr(C) ABI 兼容）==========

#[repr(C)]
pub struct __ptcb {
    pub __f: Option<extern "C" fn(*mut core::ffi::c_void)>,  // 清理回调函数指针
    pub __x: *mut core::ffi::c_void,                          // 回调参数
    pub __next: *mut __ptcb,                                  // 链表下一节点
}

// ========== 底层函数（对外导出）==========

extern "C" fn _pthread_cleanup_push(cb: *mut __ptcb,
    f: Option<extern "C" fn(*mut core::ffi::c_void)>,
    x: *mut core::ffi::c_void);

extern "C" fn _pthread_cleanup_pop(cb: *mut __ptcb,
    run: core::ffi::c_int);

// ========== 内部导出函数 ==========

extern "C" fn __do_cleanup_push(cb: *mut __ptcb);

extern "C" fn __do_cleanup_pop(cb: *mut __ptcb);
```

---

## 意图
提供线程取消清理机制：在临界区代码执行前压入清理处理函数，在临界区结束后弹出。若线程在 push/pop 之间被取消，清理栈上的所有处理函数将按 LIFO 顺序自动执行。

在 C 中，`pthread_cleanup_push` 和 `pthread_cleanup_pop` 是必须成对出现在同一词法作用域内的宏。在 Rust 中，考虑使用 RAII guard 模式替代宏，但底层 ABI 函数仍需导出以保持兼容性。

## 前置条件
- `cb` 非空，为栈分配的 `struct __ptcb` 对象
- `f` 非空（`f.is_some()`），为清理回调函数
- `_pthread_cleanup_push` 和 `_pthread_cleanup_pop` 必须配对使用
- `__do_cleanup_push` 将 `cb` 链接到当前线程的 `cancelbuf` 链表头

## 后置条件

**_pthread_cleanup_push:**
- `cb->__f = f`，`cb->__x = x`
- `__do_cleanup_push(cb)` 将 `cb` 压入当前线程的清理栈
- 若后续线程在 `_pthread_cleanup_pop` 之前被取消，`cb->__f(cb->__x)` 被自动调用

**_pthread_cleanup_pop:**
- `__do_cleanup_pop(cb)` 将 `cb` 从清理栈中移除
- 若 `run != 0`：执行 `cb->__f(cb->__x)`
- 若 `run == 0`：清理回调不被执行

## 不变量
- 每个 `pthread_cleanup_push` 必须对应一个 `pthread_cleanup_pop`，且在同一作用域内
- 清理栈符合 LIFO（后进先出）顺序

## 算法

```rust
// ========== 底层 ABI 函数 ==========

// _pthread_cleanup_push — 压入清理处理函数
pub extern "C" fn _pthread_cleanup_push(cb: *mut __ptcb,
    f: Option<extern "C" fn(*mut core::ffi::c_void)>,
    x: *mut core::ffi::c_void) {
    unsafe {
        (*cb).__f = f;
        (*cb).__x = x;
    }
    // 压入线程本地清理栈
    __do_cleanup_push(cb);
}

// _pthread_cleanup_pop — 弹出清理处理函数
pub extern "C" fn _pthread_cleanup_pop(cb: *mut __ptcb, run: core::ffi::c_int) {
    // 从清理栈中弹出
    __do_cleanup_pop(cb);
    // 若要求执行清理函数
    if run != 0 {
        unsafe {
            if let Some(f) = (*cb).__f {
                f((*cb).__x);
            }
        }
    }
}
```

---

## Rust 安全包装（模块内部，不对外暴露）

在 Rust 中，`pthread_cleanup_push/pop` 的宏模式可以替换为 RAII guard 模式：

```rust
// 线程清理栈的 RAII guard
// 使用 Drop trait 确保清理函数在作用域退出时正确弹出

pub(crate) struct CleanupGuard {
    cb: __ptcb,
    popped: bool,
}

impl CleanupGuard {
    /// 创建清理 guard，压入清理回调。
    /// 对应 C 中的 pthread_cleanup_push(f, x)。
    pub(crate) fn new(f: extern "C" fn(*mut core::ffi::c_void), x: *mut core::ffi::c_void) -> Self {
        let mut cb = __ptcb {
            __f: Some(f),
            __x: x,
            __next: core::ptr::null_mut(),
        };
        __do_cleanup_push(&mut cb);
        CleanupGuard { cb, popped: false }
    }

    /// 弹出清理回调，可选是否执行。
    /// 对应 C 中的 pthread_cleanup_pop(execute)。
    pub(crate) fn pop(mut self, execute: bool) {
        self.popped = true;
        __do_cleanup_pop(&mut self.cb);
        if execute {
            if let Some(f) = self.cb.__f {
                f(self.cb.__x);
            }
        }
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if !self.popped {
            // 若未显式 pop（如 panic/unwind），弹出但不执行清理
            __do_cleanup_pop(&mut self.cb);
        }
    }
}
```

使用示例：
```rust
// C 代码:
//   pthread_cleanup_push(cleanup, arg);
//   do_critical_work();
//   pthread_cleanup_pop(1);
//
// Rust 等效:
{
    let guard = CleanupGuard::new(cleanup, arg);
    do_critical_work();
    guard.pop(true);  // 执行清理
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  __do_cleanup_push(cb)                // 依赖1: 将 cb 压入当前线程 cancelbuf 链表头
  __do_cleanup_pop(cb)                 // 依赖2: 将 cb 从当前线程 cancelbuf 链表移除
Predefined Structures:
  __ptcb                               // 依赖3: 线程取消清理控制块（repr(C) 兼容）
  struct pthread { cancelbuf }          // 依赖4: 当前线程的清理栈链表头

[GUARANTEE]
Exported Interface:
  extern "C" fn _pthread_cleanup_push(cb: *mut __ptcb, f: ..., x: *mut core::ffi::c_void);
  extern "C" fn _pthread_cleanup_pop(cb: *mut __ptcb, run: core::ffi::c_int);
                                    // 本模块保证对外提供与 C ABI 兼容的符号
  extern "C" fn __do_cleanup_push(cb: *mut __ptcb);
  extern "C" fn __do_cleanup_pop(cb: *mut __ptcb);
                                    // 内部 hidden 符号也需导出
Internal Interface:
  pub(crate) struct CleanupGuard;   // RAII guard，替代 C 宏
  pub(crate) fn cleanup_push(f: ..., x: ...) -> CleanupGuard;
  pub(crate) fn cleanup_pop(guard: CleanupGuard, execute: bool);
                                    // 安全包装，供 crate 内部使用
