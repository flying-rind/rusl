//! 全局分配器 — 在 no_std + alloc 环境下为 Box/Vec/String 等类型提供内存分配。
//! 始终委托给 rusl 自己的 malloc/free/realloc，包括测试模式。

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

pub struct RuslAlloc;

unsafe impl Sync for RuslAlloc {}

unsafe impl GlobalAlloc for RuslAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = if layout.size() == 0 { 1 } else { layout.size() };
        crate::mallocng::malloc::malloc(size) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        crate::free::free(ptr as *mut c_void);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        let size = if new_size == 0 { 1 } else { new_size };
        crate::realloc::realloc(ptr as *mut c_void, size) as *mut u8
    }
}

// #[cfg(feature = "allocator")]
#[global_allocator] 
static GLOBAL: RuslAlloc = RuslAlloc;
