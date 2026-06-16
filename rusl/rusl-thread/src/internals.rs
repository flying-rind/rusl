//! # musl `__` 前缀内部符号桩
//!
//! musl 中 `__xxx` 是主实现函数，无前缀的 POSIX 名称通过 `weak_alias` 映射。
//! rusl 必须同时提供两者作为独立符号。

use core::ffi::{c_int, c_void};

use crate::types::*;

// ============================================================================
// 19.1 rwlock __ 符号
// ============================================================================

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 19.2 mutex __ 符号
// ============================================================================

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 19.3 其他 __ 符号
// ============================================================================

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_testcancel() {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_self_internal() -> pthread_t {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_equal(a: pthread_t, b: pthread_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_key_create(k: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_key_delete(k: pthread_key_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_getspecific(k: pthread_key_t) -> *mut c_void {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __pthread_tsd_run_dtors() {
    unimplemented!()
}

// ============================================================================
// 20. 内部导出符号 (被其他 musl 模块使用)
// ============================================================================

/// [Visibility]: Internal
#[no_mangle]
pub static mut __pthread_tsd_size: usize = 0;

/// [Visibility]: Internal
#[no_mangle]
pub static mut __pthread_tsd_main: [*mut c_void; 128] = [core::ptr::null_mut(); 128];

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __fork_handler(who: c_int) {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __cancel() -> isize {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub unsafe extern "C" fn __syscall_cp_c(nr: isize, _: ...) -> isize {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub extern "C" fn __syscall_cp_asm(
    nr: isize,
    u: isize,
    v: isize,
    w: isize,
    x: isize,
    y: isize,
    z: isize,
) -> isize {
    unimplemented!()
}

/// [Visibility]: Internal
#[no_mangle]
pub static mut __sem_open_lockptr: *mut c_int = core::ptr::null_mut();
