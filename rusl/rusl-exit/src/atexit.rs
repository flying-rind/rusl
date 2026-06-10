//! atexit — 进程退出时执行注册的函数。
//! 对应 musl `src/exit/atexit.c`。

use core::ffi::c_int;
use core::sync::atomic::{AtomicI32, Ordering};

const COUNT: usize = 32;

/// atexit 处理器条目: 函数指针 + 参数
struct Handler {
    func: unsafe extern "C" fn(*mut core::ffi::c_void),
    arg: *mut core::ffi::c_void,
}

/// 内建处理器数组 (免 malloc)
static mut BUILTIN: [Handler; COUNT] = {
    const INIT: Handler = Handler { func: dummy, arg: core::ptr::null_mut() };
    [INIT; COUNT]
};

/// 溢出链表节点
struct Overflow {
    handlers: [Handler; COUNT],
    next: *mut Overflow,
}

/// 溢出链表头
static mut OVERFLOW_HEAD: *mut Overflow = core::ptr::null_mut();

unsafe extern "C" fn dummy(_: *mut core::ffi::c_void) {}

/// 当前在 builtin 中的槽位 (下一次写入的位置)
static mut SLOT: usize = 0;

/// atexit 处理是否已完成 (防止在处理过程中注册新的处理器)
static mut FINISHED: bool = false;

/// atexit 锁 (对应 C: volatile int lock[1])
static LOCK: AtomicI32 = AtomicI32::new(0);
#[no_mangle]
pub static mut __atexit_lockptr: *const AtomicI32 = &raw const LOCK;

// ---------------------------------------------------------------------------
// LOCK / UNLOCK — 自旋锁
// ---------------------------------------------------------------------------

fn lock(l: &AtomicI32) {
    while l.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}

fn unlock(l: &AtomicI32) {
    l.store(0, Ordering::Release);
}

// ---------------------------------------------------------------------------
// __funcs_on_exit — 调用所有注册的 atexit 处理器
// ---------------------------------------------------------------------------

/// 调用所有已注册的 atexit 处理器 (按注册逆序)。
/// 由 `exit()` 调用。
#[no_mangle]
pub unsafe extern "C" fn __funcs_on_exit() {
    lock(&LOCK);

    // 调用溢出链表中的处理器 (逆序: head 是最新的)
    let mut node: *mut Overflow = unsafe { OVERFLOW_HEAD };
    while !node.is_null() {
        let handlers = unsafe { &(*node).handlers };
        // 逆序遍历本节点中的处理器
        for i in (0..COUNT).rev() {
            let f = handlers[i].func;
            let a = handlers[i].arg;
            unlock(&LOCK);
            unsafe { f(a); }
            lock(&LOCK);
        }
        node = unsafe { (*node).next };
    }

    // 调用 builtin 中的处理器 (逆序)
    let slot = unsafe { SLOT };
    for i in (0..slot).rev() {
        let h = unsafe { &BUILTIN[i] };
        let f = h.func;
        let a = h.arg;
        unlock(&LOCK);
        unsafe { f(a); }
        lock(&LOCK);
    }

    unsafe { FINISHED = true; }
    unlock(&LOCK);
}

// ---------------------------------------------------------------------------
// __cxa_finalize — 空实现
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn __cxa_finalize(_dso: *mut core::ffi::c_void) {}

// ---------------------------------------------------------------------------
// __cxa_atexit — 注册带参数的退出处理器
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn __cxa_atexit(
    func: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
    arg: *mut core::ffi::c_void,
    _dso: *mut core::ffi::c_void,
) -> c_int {
    let f = func.unwrap_or(dummy);

    lock(&LOCK);

    if unsafe { FINISHED } {
        unlock(&LOCK);
        return -1;
    }

    let slot = unsafe { SLOT };
    if slot < COUNT {
        unsafe {
            BUILTIN[slot].func = f;
            BUILTIN[slot].arg = arg;
            SLOT = slot + 1;
        }
        unlock(&LOCK);
        return 0;
    }

    // builtin 已满 — 尝试在溢出链表中查找空位
    let mut node: *mut Overflow = unsafe { OVERFLOW_HEAD };
    while !node.is_null() {
        let handlers = unsafe { &mut (*node).handlers };
        for i in 0..COUNT {
            if handlers[i].func as usize == dummy as *const() as usize {
                handlers[i].func = f;
                handlers[i].arg = arg;
                unlock(&LOCK);
                return 0;
            }
        }
        node = unsafe { (*node).next };
    }

    // 无空位 — 无法注册 (no_std 环境无 malloc)
    unlock(&LOCK);
    -1
}

// ---------------------------------------------------------------------------
// atexit — 注册无参数退出处理器 (ISO C)
// ---------------------------------------------------------------------------

unsafe extern "C" fn call(func: *mut core::ffi::c_void) {
    let f: extern "C" fn() = unsafe { core::mem::transmute(func) };
    f();
}

#[no_mangle]
pub unsafe extern "C" fn atexit(func: Option<extern "C" fn()>) -> c_int {
    let f = match func {
        Some(f) => f,
        None => {
            extern "C" fn noop() {}
            noop
        }
    };
    unsafe { __cxa_atexit(Some(call), f as *mut core::ffi::c_void, core::ptr::null_mut()) }
}
