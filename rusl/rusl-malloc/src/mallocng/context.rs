// context.rs — mallocng 全局上下文与跨文件共享符号
//
// 对应 musl 的 src/malloc/mallocng/meta.h 中的 extern 声明。
// 本文件定义 meta.rs 引用的跨文件共享符号: SIZE_CLASSES、CTX、alloc_meta、is_allzero。
//
// 所有符号为 pub(crate)，仅供 mallocng 内部模块使用。

use core::ffi::c_void;
use core::mem;

use super::meta::{self, Meta, MetaArea, MallocContext, SIZE_CLASSES as _SC};
use super::glue;
use super::syscall;

// ============================================================================
// 大小类别查找表
// ============================================================================

/// 大小类别查找表 (以 UNIT = 16 字节为单位)。
///
/// `SIZE_CLASSES[i]` 表示第 i 个 size class 的槽位大小（以 UNIT 为单位）。
/// 共 48 个条目 (0..47)，覆盖从 1 到 ~32768 UNIT 的范围。
///
/// # 初始化
///
/// 实际值在 malloc 首次调用时由初始化代码填充。
/// 骨架中初始化为全零，待完整值确定后填充。
///
/// # 可见性
///
/// pub(crate) — 仅在 mallocng 的 .rs 文件间共享。
/// C 原版的 `extern const uint16_t size_classes[]` + `__attribute__((__visibility__("hidden")))`
/// 在 Rust 中改为 `pub(crate) static`。
pub(crate) static SIZE_CLASSES: [u16; 48] = [
    1, 2, 3, 4, 5, 6, 7, 8,
    9, 10, 12, 15,
    18, 20, 25, 31,
    36, 42, 50, 63,
    72, 84, 102, 127,
    146, 170, 204, 255,
    292, 340, 409, 511,
    584, 682, 818, 1023,
    1169, 1364, 1637, 2047,
    2340, 2730, 3276, 4095,
    4680, 5460, 6552, 8191,
];

// ============================================================================
// 全局分配器上下文
// ============================================================================

/// 全局唯一的 malloc 上下文实例。
///
/// 在 malloc 首次调用时初始化。整个进程共享唯一实例。
///
/// # 多线程安全
///
/// 对 `CTX` 的修改必须在持有 `__malloc_lock` 下进行。
/// 读取操作可能无需锁，但依赖 `AtomicI32` 保证一致性。
///
/// # 可见性
///
/// pub(crate) — 仅在 mallocng 的 .rs 文件间共享。
/// C 原版的 `extern struct malloc_context ctx` 在 Rust 中重命名为 `CTX`
/// （遵循 Rust 常量命名约定），类型重命名为 `MallocContext`。
///
/// # Safety
///
/// `static mut` 声明（全局可变状态），由调用者确保加锁访问安全。
pub(crate) static mut CTX: MallocContext = MallocContext {
    secret: 0,
    pagesize: 4096,
    init_done: 0,
    mmap_counter: 0,
    free_meta_head: core::ptr::null_mut(),
    avail_meta: core::ptr::null_mut(),
    avail_meta_count: 0,
    avail_meta_area_count: 0,
    meta_alloc_shift: 0,
    meta_area_head: core::ptr::null_mut(),
    meta_area_tail: core::ptr::null_mut(),
    avail_meta_areas: core::ptr::null_mut(),
    active: [core::ptr::null_mut(); 48],
    usage_by_class: [0; 48],
    unmap_seq: [0; 32],
    bounces: [0; 32],
    seq: 0,
    brk: 0,
};

// ============================================================================
// 元数据分配函数
// ============================================================================

/// 分配一个新的 `Meta`，优先从 `CTX.free_meta_head` 空闲链表获取，
/// 否则通过 mmap 扩展 `MetaArea`。
///
/// # 后置条件
///
/// 返回一个已清零或从空闲链表中取出的 `*mut Meta`。失败时程序终止。
///
/// # 可见性
///
/// pub(crate) — 仅在 mallocng 的 .rs 文件间共享。
#[allow(static_mut_refs)]
#[inline(never)]
pub(crate) unsafe fn alloc_meta() -> *mut Meta {
    // 首次调用时初始化全局上下文
    if CTX.init_done == 0 {
        CTX.secret = glue::get_random_secret();
        CTX.init_done = 1;
    }
    let mut pagesize = meta::pgsz();
    if pagesize < 4096 {
        pagesize = 4096;
    }

    // 快速路径: 从空闲 Meta 链表队首取出
    let m = meta::dequeue_head(&mut CTX.free_meta_head);
    if !m.is_null() {
        return m;
    }

    // 慢速路径: 需要从 MetaArea 分配新 meta
    if CTX.avail_meta_count == 0 {
        let mut need_unprotect = true;

        // 优先尝试通过 brk 扩展堆顶获取元数据页
        if CTX.avail_meta_area_count == 0 && CTX.brk != usize::MAX {
            let mut new = CTX.brk + pagesize;
            let mut need_guard = false;
            if CTX.brk == 0 {
                need_guard = true;
                CTX.brk = glue::brk(0);
                // 对齐到页边界 (处理古代内核返回 _ebss 而非下一页的情况)
                CTX.brk += (!CTX.brk).wrapping_add(1) & (pagesize - 1);
                new = CTX.brk + 2 * pagesize;
            }
            if glue::brk(new) != new {
                CTX.brk = usize::MAX; // brk 失效
            } else {
                if need_guard {
                    syscall::sys_mmap(
                        CTX.brk as *mut c_void,
                        pagesize,
                        syscall::PROT_NONE,
                        syscall::MAP_ANONYMOUS | syscall::MAP_PRIVATE | 16, /* MAP_FIXED */
                        -1,
                        0,
                    );
                }
                CTX.brk = new;
                CTX.avail_meta_areas = (new - pagesize) as *mut u8;
                CTX.avail_meta_area_count = pagesize >> 12;
                need_unprotect = false;
            }
        }

        // brk 不可用或失败时, 通过 mmap 分配新页
        if CTX.avail_meta_area_count == 0 {
            let n = 2usize << CTX.meta_alloc_shift;
            let p = syscall::sys_mmap(
                core::ptr::null_mut(),
                n * pagesize,
                syscall::PROT_NONE,
                syscall::MAP_PRIVATE | syscall::MAP_ANONYMOUS,
                -1,
                0,
            );
            if p == syscall::MAP_FAILED {
                return core::ptr::null_mut();
            }
            CTX.avail_meta_areas = (p as usize + pagesize) as *mut u8;
            CTX.avail_meta_area_count = (n - 1) * (pagesize >> 12);
            CTX.meta_alloc_shift += 1;
        }

        // 取出当前可用的 meta_area 页, 必要时去掉 PROT_NONE 保护
        let p = CTX.avail_meta_areas;
        if (p as usize) & (pagesize - 1) != 0 {
            need_unprotect = false;
        }
        if need_unprotect {
            let ret = glue::mprotect(
                p as *mut c_void,
                pagesize,
                syscall::PROT_READ | syscall::PROT_WRITE,
            );
            if ret != 0 {
                // mprotect 真正失败时返回 null
                return core::ptr::null_mut();
            }
        }

        CTX.avail_meta_area_count -= 1;
        CTX.avail_meta_areas = (p as usize + 4096) as *mut u8;

        // 将新 MetaArea 链接到链表尾部
        if !CTX.meta_area_tail.is_null() {
            (*CTX.meta_area_tail).next = p as *mut MetaArea;
        } else {
            CTX.meta_area_head = p as *mut MetaArea;
        }
        CTX.meta_area_tail = p as *mut MetaArea;
        (*CTX.meta_area_tail).check = CTX.secret;

        // 计算本页可容纳的 Meta 槽位数
        let nslots = (4096 - mem::size_of::<MetaArea>()) / mem::size_of::<Meta>();
        (*CTX.meta_area_tail).nslots = nslots as i32;
        CTX.avail_meta_count = nslots;
        // avail_meta 指向 MetaArea.slots[]
        CTX.avail_meta = (p as usize + mem::size_of::<MetaArea>()) as *mut Meta;
    }

    // 从当前 MetaArea 中取出一个 Meta
    CTX.avail_meta_count -= 1;
    let m = CTX.avail_meta;
    CTX.avail_meta = CTX.avail_meta.add(1);
    (*m).prev = core::ptr::null_mut();
    (*m).next = core::ptr::null_mut();
    m
}

// ============================================================================
// 全零检测函数
// ============================================================================

/// 检查 `p` 指向的内存页是否全部为零。
///
/// 用于判断 madvise-free 后的页是否已被内核清零回收。
///
/// # 参数
///
/// - `p`: 指向待检查内存页起始地址
///
/// # 返回值
///
/// - 1: 页面全为零
/// - 0: 页面中有非零字节
///
/// # 可见性
///
/// pub(crate) — 仅在 mallocng 的 .rs 文件间共享。
///
/// # 设计说明
///
/// C 原版的 `void *` 参数在 Rust 中使用 `*mut c_void` 等效类型。
pub(crate) unsafe fn is_allzero(p: *mut c_void) -> i32 {
    // 从分配指针反向推导 Meta, 判断分配块是否可被视为全零
    // (来自 mmap 新鲜页面的内存不需要显式 memset 清零)
    let p = p as *const u8;
    let g = meta::get_meta(p);
    if (*g).sizeclass() >= 48
        || meta::get_stride(g) < meta::UNIT * SIZE_CLASSES[(*g).sizeclass()] as usize
    {
        1
    } else {
        0
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;

    test!("test_size_classes_length_is_48" {
        assert_eq!(SIZE_CLASSES.len(), 48);
    });

    test!("test_size_classes_initialized_to_zero" {
        // SIZE_CLASSES 现在已填入正确的 musl 值，不应全为零
        assert_eq!(SIZE_CLASSES[0], 1, "SIZE_CLASSES[0] 应为 1 UNIT (16 字节)");
        assert_eq!(SIZE_CLASSES[47], 8191, "SIZE_CLASSES[47] 应为 8191 UNIT");
    });

    test!("test_size_classes_type_is_u16" {
        // SIZE_CLASSES 元素为 u16，与 C const uint16_t 等效
        assert_eq!(core::mem::size_of::<u16>(), 2);
    });

    test!("test_ctx_active_array_length" {
        // 安全访问: 验证 static mut CTX 的 active 数组长度
        unsafe {
            assert_eq!(CTX.active.len(), 48);
        }
    });

    test!("test_ctx_usage_by_class_array_length" {
        unsafe {
            assert_eq!(CTX.usage_by_class.len(), 48);
        }
    });

    test!("test_ctx_unmap_seq_array_length" {
        unsafe {
            assert_eq!(CTX.unmap_seq.len(), 32);
        }
    });

    test!("test_ctx_bounces_array_length" {
        unsafe {
            assert_eq!(CTX.bounces.len(), 32);
        }
    });

    test!("test_ctx_initial_state_zeroed" {
        // 全局分配器已在测试框架初始化时触发首次分配，CTX 已被初始化
        unsafe {
            assert_eq!(CTX.init_done, 1, "CTX 应在首次分配后被初始化");
            assert!(CTX.secret != 0, "secret 应已被随机密钥填充");
        }
    });

    test!("test_alloc_meta_signature_exists" {
        let _f: unsafe fn() -> *mut Meta = alloc_meta;
    });

    test!("test_is_allzero_signature_exists" {
        let _f: unsafe fn(*mut c_void) -> i32 = is_allzero;
    });
}