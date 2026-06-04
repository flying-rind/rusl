//! 全局分配器 — 在 no_std + alloc 环境下为 Box/Vec/String 等类型提供内存分配。
//! 此时使用musl libc的malloc模块接口

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

pub struct RuslAlloc;

unsafe impl Sync for RuslAlloc {}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

unsafe impl GlobalAlloc for RuslAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = if layout.size() == 0 { 1 } else { layout.size() };
        malloc(size) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        free(ptr as *mut c_void);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        let size = if new_size == 0 { 1 } else { new_size };
        realloc(ptr as *mut c_void, size) as *mut u8
    }
}

#[global_allocator]
static GLOBAL: RuslAlloc = RuslAlloc;