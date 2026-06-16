//! # C11 线程 API 桩
//!
//! ISO C11 `<threads.h>` 线程接口:
//! thrd_*, mtx_*, cnd_*, tss_*, call_once

use core::ffi::{c_int, c_void};

use crate::types::*;

// ============================================================================
// 18.1 一次性执行
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn call_once(flag: *mut once_flag, func: Option<unsafe extern "C" fn()>) {
    unimplemented!()
}

// ============================================================================
// 18.2 条件变量
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn cnd_init(c: *mut cnd_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn cnd_destroy(c: *mut cnd_t) {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn cnd_wait(c: *mut cnd_t, m: *mut mtx_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn cnd_timedwait(c: *mut cnd_t, m: *mut mtx_t, ts: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn cnd_signal(c: *mut cnd_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn cnd_broadcast(c: *mut cnd_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 18.3 互斥锁
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn mtx_init(m: *mut mtx_t, type_: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn mtx_destroy(mtx: *mut mtx_t) {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn mtx_lock(m: *mut mtx_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn mtx_timedlock(m: *mut mtx_t, ts: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn mtx_trylock(m: *mut mtx_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn mtx_unlock(mtx: *mut mtx_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 18.4 线程管理
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn thrd_create(thr: *mut thrd_t, func: thrd_start_t, arg: *mut c_void) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn thrd_exit(result: c_int) -> ! {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn thrd_join(t: thrd_t, res: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn thrd_sleep(req: *const timespec, rem: *mut timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn thrd_yield() {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn thrd_current() -> pthread_t {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn thrd_equal(a: pthread_t, b: pthread_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 18.5 线程特定存储
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn tss_create(tss: *mut tss_t, dtor: tss_dtor_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn tss_delete(key: tss_t) {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn tss_set(k: tss_t, x: *mut c_void) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn tss_get(k: tss_t) -> *mut c_void {
    unimplemented!()
}
