//! PRNG — 伪随机数生成

use core::ffi::c_int;

pub const RAND_MAX: c_int = 0x7fffffff;

// ---------- internal FFI declarations ----------

extern "C" {
    #[link_name = "drand48"]
    fn musl_drand48() -> f64;
    #[link_name = "erand48"]
    fn musl_erand48(xsubi: *mut u16) -> f64;
    #[link_name = "srand48"]
    fn musl_srand48(seedval: i64);
    #[link_name = "lrand48"]
    fn musl_lrand48() -> i64;
    #[link_name = "nrand48"]
    fn musl_nrand48(xsubi: *mut u16) -> i64;
    #[link_name = "mrand48"]
    fn musl_mrand48() -> i64;
    #[link_name = "jrand48"]
    fn musl_jrand48(xsubi: *mut u16) -> i64;
    #[link_name = "lcong48"]
    fn musl_lcong48(p: *const u16);
    #[link_name = "seed48"]
    fn musl_seed48(seed16v: *const u16) -> *mut u16;
    #[link_name = "random"]
    fn musl_random() -> i64;
    #[link_name = "srandom"]
    fn musl_srandom(seed: u32);
    #[link_name = "initstate"]
    fn musl_initstate(seed: u32, state: *mut u8, n: usize) -> *mut u8;
    #[link_name = "setstate"]
    fn musl_setstate(state: *mut u8) -> *mut u8;
    #[link_name = "rand_r"]
    fn musl_rand_r(seed: *mut u32) -> i32;
    #[link_name = "rand"]
    fn musl_rand() -> i32;
    #[link_name = "srand"]
    fn musl_srand(seed: u32);
}

// ---------- safe public wrappers ----------

pub extern "C" fn drand48() -> f64                                 { unsafe { musl_drand48() } }
pub extern "C" fn erand48(xsubi: *mut u16) -> f64                  { unsafe { musl_erand48(xsubi) } }
pub extern "C" fn srand48(seedval: i64)                            { unsafe { musl_srand48(seedval) } }
pub extern "C" fn lrand48() -> i64                                 { unsafe { musl_lrand48() } }
pub extern "C" fn nrand48(xsubi: *mut u16) -> i64                  { unsafe { musl_nrand48(xsubi) } }
pub extern "C" fn mrand48() -> i64                                 { unsafe { musl_mrand48() } }
pub extern "C" fn jrand48(xsubi: *mut u16) -> i64                  { unsafe { musl_jrand48(xsubi) } }
pub extern "C" fn lcong48(p: *const u16)                           { unsafe { musl_lcong48(p) } }
pub extern "C" fn seed48(seed16v: *const u16) -> *mut u16          { unsafe { musl_seed48(seed16v) } }
pub extern "C" fn random() -> i64                                  { unsafe { musl_random() } }
pub extern "C" fn srandom(seed: u32)                               { unsafe { musl_srandom(seed) } }
pub extern "C" fn initstate(seed: u32, state: *mut u8, n: usize) -> *mut u8 { unsafe { musl_initstate(seed, state, n) } }
pub extern "C" fn setstate(state: *mut u8) -> *mut u8              { unsafe { musl_setstate(state) } }
pub extern "C" fn rand_r(seed: *mut u32) -> i32                    { unsafe { musl_rand_r(seed) } }
pub extern "C" fn rand() -> i32                                    { unsafe { musl_rand() } }
pub extern "C" fn srand(seed: u32)                                 { unsafe { musl_srand(seed) } }