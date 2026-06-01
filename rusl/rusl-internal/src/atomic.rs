// Atomic operations — portable implementation using core::sync::atomic.
//
// Corresponds to musl's src/internal/atomic.h + arch/*/atomic_arch.h
//
// In musl, atomics are per-arch inline assembly because C11 atomics
// weren't universally available. Rust has them in core, so we use
// them directly instead of writing per-arch asm.

#![allow(dead_code)]  // infrastructure, used by later stages

use core::sync::atomic::{AtomicI32, AtomicU64, AtomicPtr, Ordering};

// --- 32-bit integer atomics ---

#[inline]
pub fn a_cas(p: *mut i32, t: i32, s: i32) -> i32 {
    let a = unsafe { &*(p as *const AtomicI32) };
    match a.compare_exchange(t, s, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(old) => old,
        Err(old) => old,
    }
}

#[inline]
pub fn a_swap(p: *mut i32, v: i32) -> i32 {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.swap(v, Ordering::SeqCst)
}

#[inline]
pub fn a_fetch_add(p: *mut i32, v: i32) -> i32 {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.fetch_add(v, Ordering::SeqCst)
}

#[inline]
pub fn a_fetch_and(p: *mut i32, v: i32) -> i32 {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.fetch_and(v, Ordering::SeqCst)
}

#[inline]
pub fn a_fetch_or(p: *mut i32, v: i32) -> i32 {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.fetch_or(v, Ordering::SeqCst)
}

#[inline]
pub fn a_and(p: *mut i32, v: i32) {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.fetch_and(v, Ordering::SeqCst);
}

#[inline]
pub fn a_or(p: *mut i32, v: i32) {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.fetch_or(v, Ordering::SeqCst);
}

#[inline]
pub fn a_inc(p: *mut i32) {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.fetch_add(1, Ordering::SeqCst);
}

#[inline]
pub fn a_dec(p: *mut i32) {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.fetch_add(-1, Ordering::SeqCst);
}

#[inline]
pub fn a_store(p: *mut i32, v: i32) {
    let a = unsafe { &*(p as *const AtomicI32) };
    a.store(v, Ordering::SeqCst);
}

// --- 64-bit integer atomics ---

#[inline]
pub fn a_and_64(p: *mut u64, v: u64) {
    let a = unsafe { &*(p as *const AtomicU64) };
    a.fetch_and(v, Ordering::SeqCst);
}

#[inline]
pub fn a_or_64(p: *mut u64, v: u64) {
    let a = unsafe { &*(p as *const AtomicU64) };
    a.fetch_or(v, Ordering::SeqCst);
}

// --- Pointer atomics ---

#[inline]
pub fn a_cas_p<T>(p: *mut *mut T, t: *mut T, s: *mut T) -> *mut T {
    let a = unsafe { &*(p as *const AtomicPtr<T>) };
    match a.compare_exchange(t, s, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(old) => old,
        Err(old) => old,
    }
}

// --- Fences ---

#[inline]
pub fn a_barrier() {
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
}

#[inline]
pub fn a_spin() {
    core::hint::spin_loop();
}

#[inline]
pub fn a_crash() -> ! {
    // Trigger SIGSEGV by writing to address 0
    unsafe {
        core::ptr::write_volatile(0 as *mut u8, 0);
    }
    loop { core::hint::spin_loop(); }
}

// --- Bit scanning ---

/// Count trailing zeros (32-bit)
#[inline]
pub fn a_ctz_32(x: u32) -> u32 {
    x.trailing_zeros()
}

/// Count trailing zeros (64-bit)
#[inline]
pub fn a_ctz_64(x: u64) -> u32 {
    x.trailing_zeros()
}

/// Count trailing zeros (long: 32 or 64 depending on arch)
#[inline]
pub fn a_ctz_l(x: usize) -> u32 {
    x.trailing_zeros()
}

/// Count leading zeros (32-bit)
#[inline]
pub fn a_clz_32(x: u32) -> u32 {
    x.leading_zeros()
}

/// Count leading zeros (64-bit)
#[inline]
pub fn a_clz_64(x: u64) -> u32 {
    x.leading_zeros()
}