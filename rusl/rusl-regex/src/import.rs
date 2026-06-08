//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口
//!
//! 所有 extern "C" 函数都通过 safe wrapper 暴露，调用者无需使用 unsafe。

// ============================================================================
// ctype 模块：宽字符分类函数
// ============================================================================

#[cfg(not(feature = "rusl"))]
mod ctype_ffi {
    use core::ffi::{c_char, c_int};
    extern "C" {
        #[link_name = "iswalnum"]
        pub fn iswalnum(wc: u32) -> c_int;
        #[link_name = "iswalpha"]
        pub fn iswalpha(wc: u32) -> c_int;
        #[link_name = "iswblank"]
        pub fn iswblank(wc: u32) -> c_int;
        #[link_name = "iswcntrl"]
        pub fn iswcntrl(wc: u32) -> c_int;
        #[link_name = "iswdigit"]
        pub fn iswdigit(wc: u32) -> c_int;
        #[link_name = "iswgraph"]
        pub fn iswgraph(wc: u32) -> c_int;
        #[link_name = "iswlower"]
        pub fn iswlower(wc: u32) -> c_int;
        #[link_name = "iswprint"]
        pub fn iswprint(wc: u32) -> c_int;
        #[link_name = "iswpunct"]
        pub fn iswpunct(wc: u32) -> c_int;
        #[link_name = "iswspace"]
        pub fn iswspace(wc: u32) -> c_int;
        #[link_name = "iswupper"]
        pub fn iswupper(wc: u32) -> c_int;
        #[link_name = "iswxdigit"]
        pub fn iswxdigit(wc: u32) -> c_int;
        #[link_name = "towlower"]
        pub fn towlower(wc: u32) -> u32;
        #[link_name = "towupper"]
        pub fn towupper(wc: u32) -> u32;
        #[link_name = "iswctype"]
        pub fn iswctype(wc: u32, desc: u64) -> c_int;
        #[link_name = "wctype"]
        pub fn wctype(name: *const c_char) -> u64;
    }
}

#[cfg(feature = "rusl")]
mod ctype_ffi {
    pub use rusl_ctype::{
        iswalnum, iswalpha, iswblank, iswcntrl, iswdigit,
        iswgraph, iswlower, iswprint, iswpunct, iswspace,
        iswupper, iswxdigit, towlower, towupper, iswctype, wctype,
    };
}

#[allow(unused_unsafe)]
pub mod ctype {
    use core::ffi::{c_char, c_int};

    pub fn iswalnum(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswalnum(wc) } }
    pub fn iswalpha(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswalpha(wc) } }
    pub fn iswblank(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswblank(wc) } }
    pub fn iswcntrl(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswcntrl(wc) } }
    pub fn iswdigit(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswdigit(wc) } }
    pub fn iswgraph(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswgraph(wc) } }
    pub fn iswlower(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswlower(wc) } }
    pub fn iswprint(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswprint(wc) } }
    pub fn iswpunct(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswpunct(wc) } }
    pub fn iswspace(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswspace(wc) } }
    pub fn iswupper(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswupper(wc) } }
    pub fn iswxdigit(wc: u32) -> c_int { unsafe { super::ctype_ffi::iswxdigit(wc) } }
    pub fn towlower(wc: u32) -> u32 { unsafe { super::ctype_ffi::towlower(wc) } }
    pub fn towupper(wc: u32) -> u32 { unsafe { super::ctype_ffi::towupper(wc) } }
    pub fn iswctype(wc: u32, desc: u64) -> c_int { unsafe { super::ctype_ffi::iswctype(wc, desc) } }
    pub fn wctype(name: *const c_char) -> u64 { unsafe { super::ctype_ffi::wctype(name) } }

    // wctype 常量 (matches musl iswctype constants)
    pub const WCTYPE_ALNUM: u64 = 1;
    pub const WCTYPE_ALPHA: u64 = 2;
    pub const WCTYPE_BLANK: u64 = 3;
    pub const WCTYPE_CNTRL: u64 = 4;
    pub const WCTYPE_DIGIT: u64 = 5;
    pub const WCTYPE_GRAPH: u64 = 6;
    pub const WCTYPE_LOWER: u64 = 7;
    pub const WCTYPE_PRINT: u64 = 8;
    pub const WCTYPE_PUNCT: u64 = 9;
    pub const WCTYPE_SPACE: u64 = 10;
    pub const WCTYPE_UPPER: u64 = 11;
    pub const WCTYPE_XDIGIT: u64 = 12;
}

// ============================================================================
// string 模块
// ============================================================================

#[cfg(not(feature = "rusl"))]
mod string_ffi {
    use core::ffi::{c_char, c_int};
    extern "C" {
        #[link_name = "strcmp"]
        pub fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    }
}

#[cfg(feature = "rusl")]
mod string_ffi {
    pub use rusl_string::strcmp;
}

pub mod string {
    use core::ffi::{c_char, c_int};
    pub fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
        unsafe { super::string_ffi::strcmp(s1, s2) }
    }
}

// ============================================================================
// env 模块
// ============================================================================

#[cfg(not(feature = "rusl"))]
mod env_ffi {
    use core::ffi::c_char;
    extern "C" {
        #[link_name = "getenv"]
        pub fn getenv(name: *const c_char) -> *mut c_char;
    }
}

#[cfg(feature = "rusl")]
mod env_ffi {
    pub use rusl_env::getenv;
}

pub mod env {
    use core::ffi::c_char;
    pub fn getenv(name: *const c_char) -> *mut c_char {
        unsafe { super::env_ffi::getenv(name) }
    }
}
