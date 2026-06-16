# pthread_atfork — Rust 接口归约

## 原始 C 接口
```c
// ========== 对外导出函数 ==========

// POSIX 用户接口，注册 fork 处理函数
int pthread_atfork(void (*prepare)(void), void (*parent)(void), void (*child)(void));

// ========== 内部 hidden 函数 ==========

// 由 fork() 实现调用，执行注册的回调
hidden void __fork_handler(int who);

// ========== 内部 static 数据结构 ==========

static struct atfork_funcs {
    void (*prepare)(void);
    void (*parent)(void);
    void (*child)(void);
    struct atfork_funcs *prev, *next;
} *funcs;

static volatile int lock[1];
```

---

## Rust 外部 ABI 接口

```rust
// ========== 对外导出函数 ==========

extern "C" fn pthread_atfork(
    prepare: Option<extern "C" fn()>,
    parent: Option<extern "C" fn()>,
    child: Option<extern "C" fn()>,
) -> core::ffi::c_int;

// ========== 内部导出函数（hidden，被 fork() 实现调用）==========

extern "C" fn __fork_handler(who: core::ffi::c_int);
```

---

## 意图
注册在 `fork()` 之前和之后执行的回调函数：
- `prepare`：fork 前调用（通常用于获取锁，确保 fork 时无其他线程持有资源）
- `parent`：fork 后父进程中调用（释放锁）
- `child`：fork 后子进程中调用（释放/重置锁，因为子进程仅有一个线程）

## 前置条件
- `prepare`、`parent`、`child` 中至少一个非空（否则注册无意义，但不报错）
- 有足够的堆内存用于分配新节点

## 后置条件
- Case 1 内存不足（分配失败）：返回 `ENOMEM`
- Case 2 成功：新节点被插入 `funcs` 双向链表头部，回调将在后续 `fork()` 时被调用，返回 0

## 不变量
- 注册的回调在每次 `fork()` 时都会被执行，直到进程终止
- 链表节点不会被释放（进程生命周期内持久存在）
- 回调执行顺序：prepare 按注册顺序（先进先出），parent/child 按反注册顺序（后进先出）

## 算法

```rust
use core::sync::atomic::{AtomicI32, Ordering};

// ========== 内部数据结构 ==========

// atfork_funcs 节点
struct AtforkFuncs {
    prepare: Option<extern "C" fn()>,
    parent: Option<extern "C" fn()>,
    child: Option<extern "C" fn()>,
    prev: *mut AtforkFuncs,
    next: *mut AtforkFuncs,
}

// 全局链表头（static 变量）
static FUNCS: Mutex<*mut AtforkFuncs> = Mutex::new(core::ptr::null_mut());

// ========== __fork_handler — 内部导出函数 ==========

// who: -1 = prepare, 0 = parent, 1 = child
pub extern "C" fn __fork_handler(who: core::ffi::c_int) {
    let funcs = FUNCS.lock();
    let head = *funcs;

    if head.is_null() {
        return;
    }

    if who < 0 {
        // prepare 阶段：正向遍历，不释放锁（fork 期间保持锁）
        let mut p = head;
        while !p.is_null() {
            unsafe {
                if let Some(prepare) = (*p).prepare {
                    prepare();
                }
                *funcs = p;  // 更新当前节点
                p = (*p).next;
            }
        }
        // 注意：prepare 阶段不释放锁，fork 后由 parent/child 阶段释放
    } else {
        // parent/child 阶段：反向遍历
        let mut p = head;
        // 先走到链表末尾
        unsafe {
            while !(*p).next.is_null() {
                p = (*p).next;
            }
        }
        // 反向遍历
        while !p.is_null() {
            unsafe {
                if who == 0 {
                    if let Some(parent) = (*p).parent {
                        parent();
                    }
                } else {
                    if let Some(child) = (*p).child {
                        child();
                    }
                }
                *funcs = p;
                p = (*p).prev;
            }
        }
        // 释放锁
        drop(funcs);
    }
}

// ========== pthread_atfork — 对外导出函数 ==========

pub extern "C" fn pthread_atfork(
    prepare: Option<extern "C" fn()>,
    parent: Option<extern "C" fn()>,
    child: Option<extern "C" fn()>,
) -> core::ffi::c_int {
    // 分配新节点（使用内部 malloc）
    let new = libc_malloc(core::mem::size_of::<AtforkFuncs>()) as *mut AtforkFuncs;
    if new.is_null() {
        return ENOMEM;
    }

    // 初始化新节点
    unsafe {
        (*new).prepare = prepare;
        (*new).parent = parent;
        (*new).child = child;
        (*new).prev = core::ptr::null_mut();
    }

    // 加锁，插入链表头部
    let mut funcs = FUNCS.lock();
    unsafe {
        (*new).next = *funcs;
        if !(*funcs).is_null() {
            (*(*funcs)).prev = new;
        }
        *funcs = new;
    }
    // 锁在离开作用域时自动释放

    0
}
```

对 C 调用者：
1. `extern "C" fn pthread_atfork(prepare, parent, child) -> c_int`
2. 内部分配节点 → 加锁 → 插入链表头部 → 解锁
3. 返回 0 或 `ENOMEM`

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use alloc::boxed::Box;

// 安全的 Rust 封装
// 使用 Box 进行堆分配，避免手动内存管理

struct AtforkNode {
    prepare: Option<extern "C" fn()>,
    parent: Option<extern "C" fn()>,
    child: Option<extern "C" fn()>,
    prev: *mut AtforkNode,
    next: *mut AtforkNode,
}

pub(crate) fn register_atfork(
    prepare: Option<extern "C" fn()>,
    parent: Option<extern "C" fn()>,
    child: Option<extern "C" fn()>,
) -> Result<(), Errno> {
    // 使用 Box::new 分配节点
    let node = Box::new(AtforkNode {
        prepare, parent, child,
        prev: core::ptr::null_mut(),
        next: core::ptr::null_mut(),
    });
    let node_ptr = Box::into_raw(node);

    // 加锁插入链表头部
    let mut funcs = FUNCS.lock();
    unsafe {
        (*node_ptr).next = *funcs;
        if !(*funcs).is_null() {
            (*(*funcs)).prev = node_ptr;
        }
        *funcs = node_ptr;
    }
    Ok(())
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  libc_malloc(size)                     // 依赖1: 内部 malloc（libc 重定义为 __libc_malloc）
  FUNCS: Mutex<*mut AtforkFuncs>        // 依赖2: 全局链表互斥锁（内部自旋锁）
Predefined Macros/Constants:
  ENOMEM                                // 依赖3: 错误码
Predefined Structures:
  AtforkFuncs (struct atfork_funcs)     // 依赖4: fork 处理函数节点

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_atfork(prepare: ..., parent: ..., child: ...) -> core::ffi::c_int;
  extern "C" fn __fork_handler(who: core::ffi::c_int);
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_atfork 和 __fork_handler 符号
Internal Interface:
  pub(crate) fn register_atfork(prepare: ..., parent: ..., child: ...) -> Result<(), Errno>;
                                    // 安全包装，供 crate 内部使用
