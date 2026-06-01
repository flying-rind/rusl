// libc global state.
//
// Corresponds to musl's src/internal/libc.c + src/internal/libc.h
//
// The __libc struct holds all global libc state:
//   - threading flags (can_do_threads, threaded, threads_minus_1)
//   - security flag (secure for setuid/setgid)
//   - aux vector pointer
//   - TLS tracking
//   - page size
//   - global locale

use rusl_core::c_types::size_t;

// --- Forward declarations for types defined in later stages ---

// Locale map (placeholder — real definition in stage 9)
#[repr(C)]
pub struct __locale_map {
    _opaque: [u8; 64],  // placeholder size
}

// Locale struct (matches musl's struct __locale_struct)
#[repr(C)]
pub struct __locale_struct {
    pub cat: [*const __locale_map; 6],
}

// TLS module tracking
#[repr(C)]
pub struct tls_module {
    pub next: *mut tls_module,
    pub image: *mut core::ffi::c_void,
    pub len: size_t,
    pub size: size_t,
    pub align: size_t,
    pub offset: size_t,
}

// --- The main libc global state ---

#[repr(C)]
pub struct __libc {
    pub can_do_threads: core::ffi::c_char,
    pub threaded: core::ffi::c_char,
    pub secure: core::ffi::c_char,
    pub need_locks: i8,            // volatile signed char — negative = need locks
    pub threads_minus_1: core::ffi::c_int,
    pub auxv: *mut size_t,
    pub tls_head: *mut tls_module,
    pub tls_size: size_t,
    pub tls_align: size_t,
    pub tls_cnt: size_t,
    pub page_size: size_t,
    pub global_locale: __locale_struct,
}

// --- Global instance ---

// In musl, this is initialized to zero (BSS) and filled at startup by __init_libc.
// This is NOT thread-safe — there's only one instance.
#[no_mangle]
pub static mut __libc: __libc = __libc {
    can_do_threads: 0,
    threaded: 0,
    secure: 0,
    need_locks: -1,     // negative = need locks (safe default before threads init)
    threads_minus_1: 0,
    auxv: core::ptr::null_mut(),
    tls_head: core::ptr::null_mut(),
    tls_size: 0,
    tls_align: 0,
    tls_cnt: 0,
    page_size: 4096,    // default page size
    global_locale: __locale_struct {
        cat: [core::ptr::null(); 6],
    },
};

// --- hwcap (hardware capabilities from AT_HWCAP) ---

#[no_mangle]
pub static mut __hwcap: size_t = 0;

// --- Program name (set by crt/startup code) ---

#[no_mangle]
pub static mut __progname: *const core::ffi::c_char = core::ptr::null();
#[no_mangle]
pub static mut __progname_full: *const core::ffi::c_char = core::ptr::null();

// Weak aliases for GNU compatibility:
// In C: weak_alias(__progname, program_invocation_short_name);
//       weak_alias(__progname_full, program_invocation_name);
// Rust doesn't have weak symbols natively, so we export direct aliases.
// These will be used in Stage 10 (crt) startup.

// __libc_version is in version.rs
// __init_libc, __init_tls, __init_ssp are called from crt (Stage 10)