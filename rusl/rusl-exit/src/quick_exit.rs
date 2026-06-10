//! quick_exit / at_quick_exit — 快速退出机制。
//! 对应 musl `src/exit/quick_exit.c` 和 `src/exit/at_quick_exit.c`。

use core::ffi::c_int;
use core::sync::atomic::{AtomicI32, Ordering};
use super::_Exit;

const COUNT: usize = 32;

static mut FUNCS: [Option<extern "C" fn()>; COUNT] = [None; COUNT];
static mut COUNT_REGISTERED: usize = 0;
static LOCK: AtomicI32 = AtomicI32::new(0);

#[no_mangle]
pub static mut __at_quick_exit_lockptr: *const AtomicI32 = &raw const LOCK;

fn lock(l: &AtomicI32) {
    while l.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}

fn unlock(l: &AtomicI32) {
    l.store(0, Ordering::Release);
}

/// 调用所有已注册的 quick_exit 处理器 (逆序)。
#[no_mangle]
pub unsafe extern "C" fn __funcs_on_quick_exit() {
    lock(&LOCK);
    while unsafe { COUNT_REGISTERED } > 0 {
        let idx = unsafe { COUNT_REGISTERED - 1 };
        let f = unsafe { FUNCS[idx].unwrap() };
        unlock(&LOCK);
        f();
        lock(&LOCK);
        unsafe { COUNT_REGISTERED = idx; }
    }
    unlock(&LOCK);
}

/// 注册 quick_exit 处理器。
#[no_mangle]
pub unsafe extern "C" fn at_quick_exit(func: Option<extern "C" fn()>) -> c_int {
    let f = func.unwrap_or(dummy);
    lock(&LOCK);
    let cnt = unsafe { COUNT_REGISTERED };
    if cnt >= COUNT {
        unlock(&LOCK);
        return -1;
    }
    unsafe {
        FUNCS[cnt] = Some(f);
        COUNT_REGISTERED = cnt + 1;
    }
    unlock(&LOCK);
    0
}

extern "C" fn dummy() {}

/// ISO C `quick_exit` — 调用 quick_exit 处理器后终止。
#[no_mangle]
pub unsafe extern "C" fn quick_exit(code: c_int) -> ! {
    unsafe { __funcs_on_quick_exit(); }
    unsafe { _Exit(code) }
}
