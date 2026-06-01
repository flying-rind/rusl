//! Search — 哈希搜索、队列操作、线性搜索、二叉树操作

use core::ffi::{c_char, c_void};

// ---------- types ----------

pub type CmpFn = unsafe extern "C" fn(*const c_void, *const c_void) -> i32;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ENTRY {
    pub key: *mut c_char,
    pub data: *mut c_void,
}

#[repr(C)]
pub enum ACTION {
    FIND = 0,
    ENTER = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VISIT {
    preorder = 0,
    postorder = 1,
    endorder = 2,
    leaf = 3,
}

// ---------- internal FFI declarations ----------

extern "C" {
    #[link_name = "hcreate"]
    fn musl_hcreate(nel: usize) -> i32;
    #[link_name = "hdestroy"]
    fn musl_hdestroy();
    #[link_name = "hsearch"]
    fn musl_hsearch(item: ENTRY, action: ACTION) -> *mut ENTRY;
    #[link_name = "insque"]
    fn musl_insque(element: *mut c_void, pred: *const c_void);
    #[link_name = "remque"]
    fn musl_remque(element: *mut c_void);
    #[link_name = "lsearch"]
    fn musl_lsearch(key: *const c_void, base: *mut c_void, nelp: *mut usize, width: usize, compar: Option<CmpFn>) -> *mut c_void;
    #[link_name = "lfind"]
    fn musl_lfind(key: *const c_void, base: *const c_void, nelp: *mut usize, width: usize, compar: Option<CmpFn>) -> *mut c_void;
    #[link_name = "tsearch"]
    fn musl_tsearch(key: *const c_void, rootp: *mut *mut c_void, compar: Option<CmpFn>) -> *mut c_void;
    #[link_name = "tfind"]
    fn musl_tfind(key: *const c_void, rootp: *mut *const c_void, compar: Option<CmpFn>) -> *mut c_void;
    #[link_name = "tdelete"]
    fn musl_tdelete(key: *const c_void, rootp: *mut *mut c_void, compar: Option<CmpFn>) -> *mut c_void;
    #[link_name = "tdestroy"]
    fn musl_tdestroy(root: *mut c_void, free_key: Option<unsafe extern "C" fn(*mut c_void)>);
    #[link_name = "twalk"]
    fn musl_twalk(root: *const c_void, action: Option<unsafe extern "C" fn(*const c_void, VISIT, i32)>);
}

// ---------- safe public wrappers ----------

pub extern "C" fn hcreate(nel: usize) -> i32                                         { unsafe { musl_hcreate(nel) } }
pub extern "C" fn hdestroy()                                                         { unsafe { musl_hdestroy() } }
pub extern "C" fn hsearch(item: ENTRY, action: ACTION) -> *mut ENTRY                 { unsafe { musl_hsearch(item, action) } }
pub extern "C" fn insque(element: *mut c_void, pred: *const c_void)                  { unsafe { musl_insque(element, pred) } }
pub extern "C" fn remque(element: *mut c_void)                                       { unsafe { musl_remque(element) } }
pub extern "C" fn lsearch(key: *const c_void, base: *mut c_void, nelp: *mut usize, width: usize, compar: Option<CmpFn>) -> *mut c_void { unsafe { musl_lsearch(key, base, nelp, width, compar) } }
pub extern "C" fn lfind(key: *const c_void, base: *const c_void, nelp: *mut usize, width: usize, compar: Option<CmpFn>) -> *mut c_void { unsafe { musl_lfind(key, base, nelp, width, compar) } }
pub extern "C" fn tsearch(key: *const c_void, rootp: *mut *mut c_void, compar: Option<CmpFn>) -> *mut c_void { unsafe { musl_tsearch(key, rootp, compar) } }
pub extern "C" fn tfind(key: *const c_void, rootp: *mut *const c_void, compar: Option<CmpFn>) -> *mut c_void { unsafe { musl_tfind(key, rootp, compar) } }
pub extern "C" fn tdelete(key: *const c_void, rootp: *mut *mut c_void, compar: Option<CmpFn>) -> *mut c_void { unsafe { musl_tdelete(key, rootp, compar) } }
pub extern "C" fn tdestroy(root: *mut c_void, free_key: Option<unsafe extern "C" fn(*mut c_void)>) { unsafe { musl_tdestroy(root, free_key) } }
pub extern "C" fn twalk(root: *const c_void, action: Option<unsafe extern "C" fn(*const c_void, VISIT, i32)>) { unsafe { musl_twalk(root, action) } }