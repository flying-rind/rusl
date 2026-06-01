//! Malloc — 内存分配器

use core::ffi::c_void;

// ---------- internal FFI declarations ----------

extern "C" {
    #[link_name = "malloc"]
    fn musl_malloc(size: usize) -> *mut c_void;
    #[link_name = "calloc"]
    fn musl_calloc(nmemb: usize, size: usize) -> *mut c_void;
    #[link_name = "reallocarray"]
    fn musl_reallocarray(ptr: *mut c_void, nmemb: usize, size: usize) -> *mut c_void;
    #[link_name = "aligned_alloc"]
    fn musl_aligned_alloc(alignment: usize, size: usize) -> *mut c_void;
    #[link_name = "malloc_usable_size"]
    fn musl_malloc_usable_size(ptr: *mut c_void) -> usize;
}

// ---------- safe public wrappers ----------

pub extern "C" fn malloc(size: usize) -> *mut c_void                       { unsafe { musl_malloc(size) } }
pub extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void         { unsafe { musl_calloc(nmemb, size) } }
pub extern "C" fn reallocarray(ptr: *mut c_void, nmemb: usize, size: usize) -> *mut c_void { unsafe { musl_reallocarray(ptr, nmemb, size) } }
pub extern "C" fn aligned_alloc(alignment: usize, size: usize) -> *mut c_void { unsafe { musl_aligned_alloc(alignment, size) } }
pub extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize            { unsafe { musl_malloc_usable_size(ptr) } }

// ---------- sub-modules ----------

pub mod free {
    use core::ffi::c_void;
    extern "C" {
        #[link_name = "free"]
        fn musl_free(ptr: *mut c_void);
    }
    pub extern "C" fn free(ptr: *mut c_void) { unsafe { musl_free(ptr) } }
}

pub mod realloc {
    use core::ffi::c_void;
    extern "C" {
        #[link_name = "realloc"]
        fn musl_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    }
    pub extern "C" fn realloc(p: *mut c_void, n: usize) -> *mut c_void { unsafe { musl_realloc(p, n) } }
}

pub mod memalign {
    use core::ffi::c_void;
    extern "C" {
        #[link_name = "memalign"]
        fn musl_memalign(align: usize, len: usize) -> *mut c_void;
    }
    pub extern "C" fn memalign(align: usize, len: usize) -> *mut c_void { unsafe { musl_memalign(align, len) } }
}

pub mod posix_memalign {
    use core::ffi::{c_int, c_void};
    extern "C" {
        #[link_name = "posix_memalign"]
        fn musl_posix_memalign(res: *mut *mut c_void, align: usize, len: usize) -> c_int;
    }
    pub extern "C" fn posix_memalign(res: *mut *mut c_void, align: usize, len: usize) -> c_int { unsafe { musl_posix_memalign(res, align, len) } }
}