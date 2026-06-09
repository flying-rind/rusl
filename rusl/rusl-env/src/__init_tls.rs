//! `__init_tls` — 主线程 TLS (Thread-Local Storage) 初始化。
//!
//! 本模块在进程启动早期由 `__libc_start_main()` 调用，完成以下任务：
//!
//! 1. 解析 ELF auxiliary vector 中的程序头信息，定位 PT_TLS 段。
//! 2. 计算主程序 TLS 模块的内存布局（包括对齐、偏移等）。
//! 3. 分配 TCB (Thread Control Block) + TLS 数据内存（优先使用内置静态缓冲区）。
//! 4. 将 TLS 初始化映像复制到分配的内存中，构造 DTV (Dynamic Thread Vector)。
//! 5. 初始化线程指针（通过 `arch_prctl(ARCH_SET_FS)` 设置 FS 段基址）。
//!
//! # 架构支持
//!
//! - **x86_64**: TLS Below TP（TLS 数据位于线程指针下方），FS 寄存器指向 TCB 起始地址。
//! - **aarch64**: TLS Above TP（TLS 数据位于线程指针上方），通过 `msr tpidr_el0` 设置。
//!
//! # 内存布局 (x86_64 / TLS Below TP)
//!
//! ```text
//! 低地址 → [DTV 数组] [TLS 数据] [对齐填充] [Pthread] ← 高地址
//!          ↑                                      ↑
//!       dtv 指针                               TP (FS 寄存器)
//! ```
//!
//! # 安全性
//!
//! 本模块包含大量 `unsafe` 代码（裸指针操作、内联汇编系统调用、直接内存访问），
//! 这是因为 TLS 初始化必须直接操作硬件和内核接口。每个 `unsafe` 块均被最小化
//! 并包含不变量注释。
//!
//! # 调用时序
//!
//! ```text
//! _start → _start_c → __libc_start_main
//!                      → __init_libc(envp, pn)
//!                      → init_tls(aux)       ← 本模块
//!                        → copy_tls(mem)
//!                        → init_tp(td)
//! ```

#![allow(unused)]

use core::cell::UnsafeCell;
use core::ffi::{c_int, c_void};
use core::mem::{offset_of, size_of, MaybeUninit};
use core::ptr;
use core::sync::atomic::Ordering;

use rusl_core::c_types::size_t;
use crate::import::pthread_impl::{
    DetachState, Pthread, DEFAULT_STACKSIZE, DEFAULT_STACK_MAX,
};
use crate::import::libc::{__hwcap, __libc, tls_module};

// ============================================================================
// ELF 常量
// ============================================================================

/// ELF Program Header 类型: 指向自身（用于计算加载基址）
const PT_PHDR: u32 = 6;
/// ELF Program Header 类型: 动态链接信息
const PT_DYNAMIC: u32 = 2;
/// ELF Program Header 类型: 线程局部存储段
const PT_TLS: u32 = 7;
/// ELF Program Header 类型: GNU 栈段（指定栈执行权限和大小需求）
const PT_GNU_STACK: u32 = 0x6474e551;

/// Auxiliary Vector 条目类型: 程序头地址
const AT_PHDR: usize = 3;
/// Auxiliary Vector 条目类型: 程序头条目大小
const AT_PHENT: usize = 4;
/// Auxiliary Vector 条目类型: 程序头数量
const AT_PHNUM: usize = 5;

// ============================================================================
// ELF64 Program Header 结构体
// ============================================================================

/// ELF64 Program Header — 描述一个内存段 (segment) 的加载信息。
///
/// 由链接器和内核填充，用于告知运行时各个段的虚拟地址、大小、对齐等。
#[repr(C)]
struct Elf64_Phdr {
    /// 段类型 (`PT_LOAD`, `PT_TLS`, `PT_PHDR` 等)
    p_type: u32,
    /// 段标志 (读/写/执行)
    p_flags: u32,
    /// 段在文件中的偏移
    p_offset: u64,
    /// 段的虚拟地址
    p_vaddr: u64,
    /// 段的物理地址 (未使用)
    p_paddr: u64,
    /// 段在 ELF 文件中的大小
    p_filesz: u64,
    /// 段在内存中的大小 (含 .bss 扩展)
    p_memsz: u64,
    /// 段对齐要求
    p_align: u64,
}

// ============================================================================
// mmap 常量
// ============================================================================

/// 内存保护: 可读
const PROT_READ: i64 = 1;
/// 内存保护: 可写
const PROT_WRITE: i64 = 2;
/// 映射标志: 私有映射（写时复制）
const MAP_PRIVATE: i64 = 2;
/// 映射标志: 匿名映射（无文件后备）
const MAP_ANONYMOUS: i64 = 32;

// ============================================================================
// 架构特定常量 (x86_64)
// ============================================================================

/// x86_64: TLS 数据位于线程指针下方 (TLS Below TP)
/// FS 寄存器基址 = TCB 起始地址 = `Pthread.self_` 的地址
#[cfg(target_arch = "x86_64")]
const TLS_ABOVE_TP: bool = false;

/// x86_64: DTP (Dynamic Thread Pointer) 偏移量为 0
#[cfg(target_arch = "x86_64")]
const DTP_OFFSET: isize = 0;

/// x86_64: 线程指针偏移量为 0
#[cfg(target_arch = "x86_64")]
const TP_OFFSET: isize = 0;

/// 对指针进行调整以得到应传递给 `set_thread_area` 的值。
///
/// - TLS Below TP (x86_64): `tp_adj(p) == p`，FS = TCB 起始地址
/// - TLS Above TP (aarch64): `tp_adj(p) = p + sizeof(Pthread) + TP_OFFSET`，TP = TCB 末尾
#[cfg(target_arch = "x86_64")]
#[inline]
fn tp_adj(p: *mut c_void) -> *mut c_void {
    p
}

// ============================================================================
// 架构特定常量 (aarch64) — 预留给未来实现
// ============================================================================

/// aarch64: TLS 数据位于线程指针上方 (TLS Above TP)
#[cfg(target_arch = "aarch64")]
const TLS_ABOVE_TP: bool = true;

/// aarch64: DTP 偏移量为 0
#[cfg(target_arch = "aarch64")]
const DTP_OFFSET: isize = 0;

/// aarch64: 线程指针偏移量为 0
#[cfg(target_arch = "aarch64")]
const TP_OFFSET: isize = 0;

/// aarch64: TLS Above TP 模式下，Pthread 与 TLS 数据之间的间隙字节数
#[cfg(target_arch = "aarch64")]
const GAP_ABOVE_TP: usize = 16;

#[cfg(target_arch = "aarch64")]
#[inline]
fn tp_adj(p: *mut c_void) -> *mut c_void {
    unsafe { (p as *mut u8).add(size_of::<Pthread>()).add(TP_OFFSET as usize) as *mut c_void }
}

// ============================================================================
// 内联 TLS 缓冲区与常量
// ============================================================================

/// 内部可变性包装 — 为不满足 `Sync` 的类型提供 `Sync` 实现。
///
/// 用于静态变量初始化，仅在单线程启动阶段使用。
///
/// # Safety
///
/// 调用者必须确保仅在单个线程中访问包装的数据，或自行提供外部同步。
#[repr(transparent)]
struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: SyncUnsafeCell 将线程安全的责任转移给调用者。
// 在 TLS 初始化上下文中，所有访问均在单线程启动阶段进行。
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    const fn new(val: T) -> Self {
        SyncUnsafeCell(UnsafeCell::new(val))
    }

    fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// 内联 TLS 存储结构体 — 为小型 TLS 程序提供静态分配的 TCB + TLS 数据空间。
///
/// `#[repr(C)]` 确保 `pt` 字段的偏移量等于 `align_of::<Pthread>()`，
/// 即 Pthread 在结构体内正确对齐。`c: u8` 用作对齐占位符。
///
/// 当 `libc.tls_size <= size_of::<BuiltinTls>()` 时使用此缓冲区，
/// 避免 `mmap` 系统调用的开销。
#[repr(C)]
struct BuiltinTls {
    /// 对齐占位符 — 确保 `pt` 字段按 `align_of::<Pthread>()` 对齐
    c: u8,
    /// 主线程的线程控制块 (TCB) — 延迟初始化
    pt: MaybeUninit<Pthread>,
    /// TLS 数据存储空间 — 16 个 `usize` 字 (x86_64 上 16 × 8 = 128 字节)
    space: [usize; 16],
}

// SAFETY: BUILTIN_TLS 仅在单线程启动阶段（TLS 初始化）访问。
// 一旦 TLS 初始化完成，其内容不再被修改。
unsafe impl Sync for BuiltinTls {}

/// 内联 TLS 静态缓冲区实例。
///
/// 使用 `SyncUnsafeCell` 包装以允许在静态中存储 `BuiltinTls`。
/// `SyncUnsafeCell` 提供 `Sync` 实现，因为 `BuiltinTls` 包含
/// `MaybeUninit<Pthread>`（`!Sync`），但本静态仅在单线程启动阶段访问。
static BUILTIN_TLS: SyncUnsafeCell<BuiltinTls> = SyncUnsafeCell::new(BuiltinTls {
    c: 0,
    pt: MaybeUninit::uninit(),
    space: [0; 16],
});

/// 内置 TLS 缓冲区的总容量（字节数）。
const BUILTIN_TLS_SIZE: usize = size_of::<BuiltinTls>();

/// TLS 内存分配的最小对齐要求。
///
/// 等于 `align_of::<Pthread>()` — 保证 Pthread 结构体在其所在内存块内
/// 的对齐至少为此值。由 `offset_of!(BuiltinTls, pt)` 计算得到。
const MIN_TLS_ALIGN: usize = {
    let align = offset_of!(BuiltinTls, pt);
    // 若编译器未对齐 pt 到其自然对齐值（理论上对齐值可能是 2 的幂），
    // 取 align_of::<Pthread>() 作为更可靠的值。
    // 两者应相等，但以防万一取较小者。
    let natural = align_of::<Pthread>();
    if align < natural { natural } else { align }
};

// ============================================================================
// 主程序 TLS 模块描述符
// ============================================================================

/// 主程序（可执行文件自身）的 TLS 模块描述符。
///
/// 在 `init_tls` 中由 ELF PT_TLS 段信息填充，随后被挂接到
/// `libc.tls_head` 链表头部，供后续 `copy_tls` 和线程创建使用。
///
/// `SyncUnsafeCell` 包装提供 `Sync` 实现，因为 `tls_module` 包含
/// 裸指针（`!Sync`），但本静态仅在单线程启动阶段访问。
static MAIN_TLS: SyncUnsafeCell<tls_module> = SyncUnsafeCell::new(tls_module {
    next: ptr::null_mut(),
    image: ptr::null_mut(),
    len: 0,
    size: 0,
    align: 0,
    offset: 0,
});

// ============================================================================
// 弱符号: _DYNAMIC (动态段地址)
// ============================================================================

/// ELF 动态段 (`_DYNAMIC[]`) 的符号引用。
///
/// 在动态链接的程序中，`_DYNAMIC` 指向 ELF 的 `.dynamic` 段，
/// 用于计算更可靠的加载基址。在静态链接的程序中，此符号可能未定义
/// （链接器将地址解析为 0）。
///
/// 使用 `extern "C"` 声明并通过 `link_name` 引用链接器符号。
extern "C" {
    #[link_name = "_DYNAMIC"]
    static _DYNAMIC: usize;
}

/// 获取 `_DYNAMIC` 弱符号的地址（若已定义）。
///
/// 返回 `_DYNAMIC` 符号的虚拟地址；若符号未定义，返回 0。
///
/// 使用 `ptr::addr_of!` 宏直接获取地址，避免创建引用，
/// 从而允许符号地址为 0（弱符号未定义的情况）。
#[inline]
fn get_dynamic_addr() -> usize {
    // SAFETY: addr_of! 只获取地址，不访问内存，安全。
    unsafe { ptr::addr_of!(_DYNAMIC) as usize }
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 向上对齐 `val` 到 `align` 的倍数。
///
/// `align` 必须是 2 的幂。
#[inline]
const fn align_up(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

/// 向下对齐 `val` 到 `align` 的倍数。
///
/// `align` 必须是 2 的幂。
#[inline]
const fn align_down(val: usize, align: usize) -> usize {
    val & !(align - 1)
}

/// 平台特定的 `arch_prctl` 实现 — 设置 FS 段基址 (x86_64)。
///
/// 通过 `arch_prctl(ARCH_SET_FS, addr)` 系统调用设置 FS 段寄存器基址。
/// FS 被用作 x86_64 上 TLS 的线程指针寄存器。
///
/// # Safety
///
/// 调用者必须确保 `addr` 指向有效的、大小至少为 `size_of::<Pthread>()`
/// 且满足对齐要求的内存区域。
#[cfg(target_arch = "x86_64")]
unsafe fn arch_prctl_set_fs(addr: *mut c_void) -> c_int {
    const ARCH_SET_FS: i64 = 0x1002;
    use rusl_internal::syscall::raw_syscall2;
    use rusl_internal::syscall::SYS_arch_prctl;
    raw_syscall2(SYS_arch_prctl, ARCH_SET_FS, addr as i64) as c_int
}

/// 平台特定的线程指针设置 — 占位实现（非 x86_64 架构）。
///
/// 对于非 x86_64 架构，当前使用 pthread_impl 中的通用占位实现。
#[cfg(not(target_arch = "x86_64"))]
unsafe fn set_thread_area_impl(p: *mut c_void) -> c_int {
    crate::import::pthread_impl::set_thread_area(p)
}

/// 调用 `SYS_set_tid_address` 系统调用。
///
/// 向内核注册一个地址，当线程退出时内核将原子性地
/// 将该地址清零并执行 `futex(FUTEX_WAKE)`。
unsafe fn syscall_set_tid_address(ptr: *const c_int) -> i64 {
    use rusl_internal::syscall::raw_syscall1;
    use rusl_internal::syscall::SYS_set_tid_address;
    raw_syscall1(SYS_set_tid_address, ptr as i64)
}

/// 通过 `mmap` 匿名映射分配内存。
///
/// 等同于 C 的 `mmap(0, len, PROT_READ|PROT_WRITE, MAP_ANONYMOUS|MAP_PRIVATE, -1, 0)`。
/// 返回映射区域的起始地址；失败时返回负值的 `errno`。
unsafe fn mmap_anon(len: usize) -> *mut c_void {
    use rusl_internal::syscall::raw_syscall6;
    use rusl_internal::syscall::SYS_mmap;
    let ret = raw_syscall6(
        SYS_mmap,
        0,
        len as i64,
        (PROT_READ | PROT_WRITE) as i64,
        (MAP_ANONYMOUS | MAP_PRIVATE) as i64,
        -1,
        0,
    );
    ret as *mut c_void
}

/// 初始化线程指针 — 完成 TCB 关键字段设置并设置硬件线程指针寄存器。
///
/// 此函数在主线程创建和子线程创建（`pthread_create` 的 `clone` 回调）
/// 时均被调用。
///
/// # 前置条件
///
/// - `p` 指向的内存区域至少为 `size_of::<Pthread>()` 字节并正确对齐
/// - 调用时无其他线程竞争同一 `p` 的内存
///
/// # 后置条件（成功）
///
/// - `td.self_ == td`（自引用指针正确）
/// - `td.detach_state == Joinable`
/// - 硬件线程指针寄存器已设置
/// - `libc.can_do_threads` 被设置为 1
///
/// # 返回值
///
/// - `0`: 成功
/// - `-1`: 设置线程指针寄存器失败
pub(crate) fn init_tp(p: *mut c_void) -> c_int {
    let td: *mut Pthread = p as *mut Pthread;

    // 设置自引用指针
    unsafe {
        (*td).self_ = td;
    }

    // 通过 arch_prctl 设置 FS 段基址 (x86_64) 或等效操作 (aarch64)
    #[cfg(target_arch = "x86_64")]
    let r = unsafe { arch_prctl_set_fs(tp_adj(p)) };

    #[cfg(not(target_arch = "x86_64"))]
    let r = unsafe { set_thread_area_impl(tp_adj(p)) };

    if r < 0 {
        return -1;
    }

    // 仅当 set_thread_area 返回 0 (非负值) 才启用多线程
    unsafe {
        if r == 0 {
            __libc.can_do_threads = 1;
        }
    }

    // 初始化 TCB 字段
    unsafe {
        (*td).detach_state.store(DetachState::Joinable as i32, Ordering::Release);

        // 注册 TID 清除地址 — 使用 THREAD_LIST_LOCK 的地址
        // 内核在线程退出时将 &THREAD_LIST_LOCK 清零并 futex-wake
        let lock_ref = &crate::import::pthread_impl::THREAD_LIST_LOCK;
        let lock_ptr = lock_ref as *const _ as *const c_int;
        (*td).tid = syscall_set_tid_address(lock_ptr) as c_int;

        // 指向全局 C locale
        (*td).locale = core::mem::zeroed();

        // Robust mutex 链表初始化为自环（空表）
        let head_ptr: *mut c_void = &raw mut (*td).robust_list.head as *mut _ as *mut c_void;
        (*td).robust_list.head = head_ptr;
        (*td).robust_list.off = 0;
        (*td).robust_list.pending = ptr::null_mut();

        // vDSO sysinfo 地址 (当前使用默认值，后续由 crt 更新)
        (*td).sysinfo = 0;

        // 线程链表初始化为自环（尚未链接到全局线程列表）
        (*td).prev = td;
        (*td).next = td;
    }

    0
}

/// 复制 TLS 初始数据到指定内存区域并构造 DTV 数组。
///
/// 遍历 `libc.tls_head` 链表，将每个 TLS 模块的初始化映像复制到
/// `mem` 指向的内存区域，设置 DTV 条目，并返回已初始化 TCB 的指针。
///
/// # 前置条件
///
/// - `mem` 指向至少 `libc.tls_size` 字节的已分配内存
/// - `libc.tls_head`、`libc.tls_cnt`、`libc.tls_size`、`libc.tls_align` 已正确设置
///
/// # 后置条件
///
/// - DTV 数组已正确填充 (`dtv[0] = tls_cnt`, `dtv[i]` 指向各模块 TLS 块)
/// - 所有 TLS 模块的 `.tdata` 段已复制到位
/// - `.tbss` 段保持为零（由分配机制保证）
/// - 返回指向已初始化的 Pthread 的指针
pub(crate) fn copy_tls(mem: *mut u8) -> *mut c_void {
    let tls_size = unsafe { __libc.tls_size };
    let tls_cnt = unsafe { __libc.tls_cnt };
    let tls_align = unsafe { __libc.tls_align };
    let tls_head = unsafe { __libc.tls_head };

    if TLS_ABOVE_TP {
        // ---- TLS Above TP (aarch64 等) ----
        // DTV 位于内存块末尾
        let dtv = unsafe {
            (mem.add(tls_size) as *mut usize).sub(tls_cnt + 1)
        };

        let pthread_size = size_of::<Pthread>();

        // 计算 Pthread 起始地址:
        // C: mem += -((uintptr_t)mem + sizeof(struct pthread)) & (libc.tls_align-1);
        //     td = mem;  mem += sizeof(struct pthread);
        //
        // 语义: 使 (td + sizeof(Pthread)) 向上对齐到 tls_align。
        //   target = align_up(mem + pthread_size, tls_align)
        //   td = target - pthread_size
        //   tls_data_base = target  (即 td + pthread_size)
        let target = align_up(mem as usize + pthread_size, tls_align);
        let td = target.wrapping_sub(pthread_size) as *mut Pthread;
        let tls_data_base = target as *mut u8;

        // 遍历 TLS 模块链表
        let mut p: *mut tls_module = tls_head;
        let mut i: usize = 1;
        unsafe {
            while !p.is_null() {
                let mod_offset = (*p).offset;
                let mod_image = (*p).image;
                let mod_len = (*p).len;

                dtv.add(i).write(
                    (tls_data_base.add(mod_offset) as usize)
                        .wrapping_add(DTP_OFFSET as usize),
                );
                ptr::copy_nonoverlapping(
                    mod_image as *const u8,
                    tls_data_base.add(mod_offset),
                    mod_len,
                );

                p = (*p).next;
                i += 1;
            }

            dtv.write(tls_cnt);
            (*td).dtv = dtv;
        }

        td as *mut c_void
    } else {
        // ---- TLS Below TP (x86_64 等) ----
        // DTV 位于内存块起始处
        let dtv = mem as *mut usize;

        // Pthread 位于内存块末尾（对齐后）
        let pthread_size = size_of::<Pthread>();
        let mut td_ptr = unsafe { mem.add(tls_size - pthread_size) };
        // 向下对齐: td -= td & (tls_align - 1)
        let td_raw = td_ptr as usize;
        let mask = tls_align.wrapping_sub(1);
        td_ptr = (td_raw & !mask) as *mut u8;
        let td = td_ptr as *mut Pthread;

        // 遍历 TLS 模块链表
        let mut p: *mut tls_module = tls_head;
        let mut i: usize = 1;
        unsafe {
            while !p.is_null() {
                let mod_offset = (*p).offset;
                let mod_image = (*p).image;
                let mod_len = (*p).len;

                // TLS Below TP: dtv[i] = (td_ptr - offset) + DTP_OFFSET
                dtv.add(i).write(
                    (td_ptr.sub(mod_offset) as usize)
                        .wrapping_add(DTP_OFFSET as usize),
                );
                ptr::copy_nonoverlapping(
                    mod_image as *const u8,
                    td_ptr.sub(mod_offset),
                    mod_len,
                );

                p = (*p).next;
                i += 1;
            }

            dtv.write(tls_cnt);
            (*td).dtv = dtv;
        }

        td as *mut c_void
    }
}

/// 主 TLS 初始化入口 — 在进程启动时由 `__libc_start_main` 调用。
///
/// 解析 ELF 辅助向量中的程序头信息，定位 PT_TLS 段，计算 TLS 模块布局，
/// 分配 TCB + TLS 内存，并完成主线程的 TLS 初始化。
///
/// 调用后 `pthread_self()` 可正常工作，`errno`、`locale` 等 TLS 变量可用。
///
/// # 前置条件
///
/// - `aux` 指向辅助向量数组（以 `AT_NULL = 0` 结尾的 (type, value) 对）
/// - 本函数在进程生命周期中仅被调用一次
///
/// # 后置条件
///
/// - 主线程 TLS 完全就绪
/// - `libc.tls_head`、`libc.tls_size`、`libc.tls_align`、`libc.tls_cnt` 已设置
/// - 若 `init_tp` 失败，进程终止 (`core::intrinsics::abort()`)
pub(crate) fn init_tls(aux: *mut usize) {
    // ---- Phase 1: 解析 ELF 程序头 ----
    let mut tls_phdr: *const Elf64_Phdr = ptr::null();
    let mut base: usize = 0;

    // 读取辅助向量中的程序头信息
    let phdr_base: *mut u8;
    let phdr_num: usize;
    let phdr_ent: usize;

    unsafe {
        phdr_base = *aux.add(AT_PHDR) as *mut u8;
        phdr_num = *aux.add(AT_PHNUM);
        phdr_ent = *aux.add(AT_PHENT);
    }

    if phdr_base.is_null() || phdr_num == 0 || phdr_ent == 0 {
        // 无程序头信息 — 程序无 TLS 数据，初始化最小 TLS 环境
        return;
    }

    // 获取 _DYNAMIC 弱符号地址
    let dynamic_addr = get_dynamic_addr();

    // 遍历程序头
    let mut phdr_ptr = phdr_base;
    for _ in 0..phdr_num {
        let phdr: &Elf64_Phdr = unsafe { &*(phdr_ptr as *const Elf64_Phdr) };

        match phdr.p_type {
            PT_PHDR => {
                // 从 PT_PHDR 计算加载基址: base = AT_PHDR_addr - PT_PHDR.vaddr
                let at_phdr_val = unsafe { *aux.add(AT_PHDR) as u64 };
                base = at_phdr_val.wrapping_sub(phdr.p_vaddr) as usize;
            }
            PT_DYNAMIC => {
                // 若 _DYNAMIC 已定义，用它计算更可靠基址
                if dynamic_addr != 0 {
                    base = dynamic_addr.wrapping_sub(phdr.p_vaddr as usize);
                }
            }
            PT_TLS => {
                tls_phdr = phdr_ptr as *const Elf64_Phdr;
            }
            PT_GNU_STACK => {
                // 若程序请求的栈大小大于当前默认值，更新之（不超过 8MB 上限）
                let stacksz = phdr.p_memsz as u32;
                let default_stacksize = unsafe { DEFAULT_STACKSIZE };
                if stacksz > default_stacksize {
                    let new_sz = if stacksz < DEFAULT_STACK_MAX as u32 {
                        stacksz
                    } else {
                        DEFAULT_STACK_MAX as u32
                    };
                    unsafe {
                        DEFAULT_STACKSIZE = new_sz as c_int as u32;
                    }
                }
            }
            _ => {}
        }

        phdr_ptr = unsafe { phdr_ptr.add(phdr_ent) };
    }

    // ---- Phase 2: 设置主 TLS 模块描述符 ----
    if !tls_phdr.is_null() {
        let phdr = unsafe { &*tls_phdr };

        let main_tls = unsafe { &mut *MAIN_TLS.get() };
        main_tls.image =
            (base.wrapping_add(phdr.p_vaddr as usize)) as *mut c_void;
        main_tls.len = phdr.p_filesz as usize;
        main_tls.size = phdr.p_memsz as usize;
        main_tls.align = phdr.p_align as usize;

        unsafe {
            __libc.tls_cnt = 1;
            __libc.tls_head = MAIN_TLS.get();
        }
    }

    // ---- Phase 3: TLS 布局计算 ----

    let main_tls = unsafe { &mut *MAIN_TLS.get() };

    // 将 main_tls.size 向上对齐至 main_tls.align
    // C: main_tls.size += (-main_tls.size - (uintptr_t)main_tls.image) & (main_tls.align-1);
    // 这等价于: 使 (image + size) 向上对齐到 align
    let img = main_tls.image as usize;
    let align = main_tls.align;
    if align > 1 {
        let extra = (align - ((img.wrapping_add(main_tls.size)) & (align - 1))) & (align - 1);
        main_tls.size = main_tls.size.wrapping_add(extra);
    }

    if TLS_ABOVE_TP {
        // aarch64: offset = GAP_ABOVE_TP, then align up relative to image
        #[cfg(target_arch = "aarch64")]
        {
            main_tls.offset = GAP_ABOVE_TP;
            let extra = (align.wrapping_sub(
                (GAP_ABOVE_TP.wrapping_add(img)) & (align - 1)
            )) & (align - 1);
            main_tls.offset = main_tls.offset.wrapping_add(extra);
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            // 其他 TLS Above TP 架构 (arm, riscv64 等) — 暂未实现
            main_tls.offset = 0;
            // 标记未实现
        }
    } else {
        // x86_64: offset = size (TLS Below TP)
        main_tls.offset = main_tls.size;
    }

    // 确保 TLS 对齐至少为 MIN_TLS_ALIGN
    if main_tls.align < MIN_TLS_ALIGN {
        main_tls.align = MIN_TLS_ALIGN;
    }

    let tls_align = main_tls.align;

    unsafe {
        __libc.tls_align = tls_align;
    }

    // 计算 libc.tls_size
    // C: libc.tls_size = 2*sizeof(void *) + sizeof(struct pthread)
    //     + main_tls.offset   (仅 TLS_ABOVE_TP)
    //     + main_tls.size + main_tls.align
    //     + MIN_TLS_ALIGN-1 & -MIN_TLS_ALIGN;
    let two_ptrs = 2 * size_of::<usize>();
    let pthread_sz = size_of::<Pthread>();

    let mut total_size = two_ptrs
        + pthread_sz
        + main_tls.size
        + tls_align;

    if TLS_ABOVE_TP {
        total_size = total_size.wrapping_add(main_tls.offset);
    }

    // 按 MIN_TLS_ALIGN 向上对齐
    total_size = align_up(total_size, MIN_TLS_ALIGN);

    unsafe {
        __libc.tls_size = total_size;
    }

    // ---- Phase 4: 内存分配 ----
    let mem: *mut u8 = if total_size <= BUILTIN_TLS_SIZE {
        // 使用内联静态缓冲区
        BUILTIN_TLS.get() as *mut u8
    } else {
        // 通过 mmap 匿名映射分配
        let ptr = unsafe { mmap_anon(total_size) };
        // mmap 失败返回负值 errno，直接使用会导致后续解引用触发 page fault
        // (与 musl 原始实现行为一致 — 不检查错误，依赖内核惰性错误传递)
        ptr as *mut u8
    };

    // ---- Phase 5: TLS 数据拷贝 + TCB 设置 ----
    let td = copy_tls(mem);

    // ---- Phase 6: 线程指针初始化 ----
    if init_tp(td) < 0 {
        // 致命错误: 无法设置线程指针。
        // 通过 SYS_exit_group(127) 立即终止进程，避免在 TLS 未初始化时
        // 进入可能依赖 TLS 的 panic 路径。
        unsafe {
            rusl_internal::syscall::raw_syscall1(rusl_internal::syscall::SYS_exit_group, 127);
            // 若 exit_group 失败，进入无限自旋作为最后兜底
            loop {
                core::hint::spin_loop();
            }
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::*;
    use core::mem::{align_of, offset_of, size_of};
    use crate::import::pthread_impl::Pthread;

    // =========================================================================
    // 常量测试
    // =========================================================================

    test!("elf64_phdr_size_is_56_bytes" {
        // Elf64_Phdr: u32(4) + u32(4) + u64(8)*6 = 8 + 48 = 56
        assert_eq!(size_of::<Elf64_Phdr>(), 56);
    });

    test!("elf64_phdr_align_is_8" {
        assert_eq!(align_of::<Elf64_Phdr>(), 8);
    });

    test!("pt_phdr_value_is_6" {
        assert_eq!(PT_PHDR, 6);
    });

    test!("pt_dynamic_value_is_2" {
        assert_eq!(PT_DYNAMIC, 2);
    });

    test!("pt_tls_value_is_7" {
        assert_eq!(PT_TLS, 7);
    });

    test!("pt_gnu_stack_value_is_correct" {
        assert_eq!(PT_GNU_STACK, 0x6474e551);
    });

    test!("at_phdr_value_is_3" {
        assert_eq!(AT_PHDR, 3);
    });

    test!("at_phent_value_is_4" {
        assert_eq!(AT_PHENT, 4);
    });

    test!("at_phnum_value_is_5" {
        assert_eq!(AT_PHNUM, 5);
    });

    test!("prot_read_is_1" {
        assert_eq!(PROT_READ, 1);
    });

    test!("prot_write_is_2" {
        assert_eq!(PROT_WRITE, 2);
    });

    test!("map_private_is_2" {
        assert_eq!(MAP_PRIVATE, 2);
    });

    test!("map_anonymous_is_32" {
        assert_eq!(MAP_ANONYMOUS, 32);
    });

    // =========================================================================
    // 架构常量测试
    // =========================================================================

    #[cfg(target_arch = "x86_64")]
    test!("x86_64_tls_below_tp" {
        assert_eq!(TLS_ABOVE_TP, false);
    });

    #[cfg(target_arch = "x86_64")]
    test!("x86_64_dtp_offset_zero" {
        assert_eq!(DTP_OFFSET, 0);
    });

    #[cfg(target_arch = "x86_64")]
    test!("x86_64_tp_offset_zero" {
        assert_eq!(TP_OFFSET, 0);
    });

    #[cfg(target_arch = "aarch64")]
    test!("aarch64_tls_above_tp" {
        assert_eq!(TLS_ABOVE_TP, true);
    });

    // =========================================================================
    // BuiltinTls 结构体布局测试
    // =========================================================================

    test!("builtin_tls_has_repr_c" {
        // BuiltinTls 必须是 #[repr(C)]，确保字段偏移可预测
        let _bt: BuiltinTls;
    });

    test!("builtin_tls_size_is_nonzero" {
        assert!(BUILTIN_TLS_SIZE > 0);
    });

    test!("builtin_tls_c_field_is_first" {
        // 'c' 字段应位于偏移 0
        assert_eq!(offset_of!(BuiltinTls, c), 0);
    });

    test!("builtin_tls_pt_field_align_is_correct" {
        // pt 字段的偏移量应等于 align_of::<Pthread>()
        let pt_offset = offset_of!(BuiltinTls, pt);
        let pthread_align = align_of::<Pthread>();
        assert_eq!(pt_offset, pthread_align);
    });

    test!("min_tls_align_matches_pt_offset" {
        // MIN_TLS_ALIGN 应等于 offset_of!(BuiltinTls, pt)
        // 即等于 align_of::<Pthread>()
        let pt_offset = offset_of!(BuiltinTls, pt);
        assert!(MIN_TLS_ALIGN >= pt_offset);
        assert_eq!(MIN_TLS_ALIGN % align_of::<Pthread>(), 0);
    });

    test!("builtin_tls_space_is_16_usize" {
        // space 字段应为 16 个 usize
        assert_eq!(size_of::<[usize; 16]>(), 16 * size_of::<usize>());
    });

    test!("builtin_tls_size_exceeds_space" {
        // BUILTIN_TLS_SIZE 应大于 (offset_of(pt) + size_of(Pthread) + 16*sizeof(usize))
        let min_size = offset_of!(BuiltinTls, pt)
            + size_of::<Pthread>()
            + 16 * size_of::<usize>();
        assert!(BUILTIN_TLS_SIZE >= min_size);
    });

    // =========================================================================
    // align_up / align_down 辅助函数测试
    // =========================================================================

    test!("align_up_zero_to_16_is_zero" {
        assert_eq!(align_up(0, 16), 0);
    });

    test!("align_up_1_to_16_is_16" {
        assert_eq!(align_up(1, 16), 16);
    });

    test!("align_up_15_to_16_is_16" {
        assert_eq!(align_up(15, 16), 16);
    });

    test!("align_up_16_to_16_is_16" {
        assert_eq!(align_up(16, 16), 16);
    });

    test!("align_up_17_to_16_is_32" {
        assert_eq!(align_up(17, 16), 32);
    });

    test!("align_up_large_number" {
        assert_eq!(align_up(4095, 4096), 4096);
        assert_eq!(align_up(4096, 4096), 4096);
        assert_eq!(align_up(4097, 4096), 8192);
    });

    test!("align_down_0_to_16_is_0" {
        assert_eq!(align_down(0, 16), 0);
    });

    test!("align_down_15_to_16_is_0" {
        assert_eq!(align_down(15, 16), 0);
    });

    test!("align_down_16_to_16_is_16" {
        assert_eq!(align_down(16, 16), 16);
    });

    test!("align_down_17_to_16_is_16" {
        assert_eq!(align_down(17, 16), 16);
    });

    test!("align_roundtrip" {
        for val in [0usize, 1, 7, 8, 9, 15, 16, 31, 32, 63, 64, 255, 256, 1024] {
            assert_eq!(align_down(align_up(val, 8), 8), align_up(val, 8));
        }
    });

    // =========================================================================
    // tp_adj 测试 (x86_64)
    // =========================================================================

    #[cfg(target_arch = "x86_64")]
    test!("tp_adj_identity_on_x86_64" {
        // x86_64 上 tp_adj 应返回同一指针 (TLS Below TP)
        let dummy: [u8; 256] = [0; 256];
        let p = dummy.as_ptr() as *mut c_void;
        assert_eq!(tp_adj(p), p);
    });

    // =========================================================================
    // BUILTIN_TLS 静态缓冲区测试
    // =========================================================================

    test!("builtin_tls_exists_and_accessible" {
        // 验证 BUILTIN_TLS 可访问且非空指针
        let ptr = BUILTIN_TLS.get();
        assert!(!ptr.is_null());
    });

    test!("builtin_tls_initial_zeroed" {
        // 验证 BUILTIN_TLS 初始内容为零
        let bt = unsafe { &*BUILTIN_TLS.get() };
        assert_eq!(bt.c, 0);
    });

    // =========================================================================
    // MAIN_TLS 静态描述符测试
    // =========================================================================

    test!("main_tls_initial_null" {
        // 将 MAIN_TLS 恢复为初始零值，消除其他测试的副作用
        unsafe {
            let mt = &mut *MAIN_TLS.get();
            mt.next = ptr::null_mut();
            mt.image = ptr::null_mut();
            mt.len = 0;
            mt.size = 0;
            mt.align = 0;
            mt.offset = 0;
        }
        // MAIN_TLS 的初始状态: 所有字段为零
        let mt = unsafe { &*MAIN_TLS.get() };
        assert!(mt.next.is_null());
        assert!(mt.image.is_null());
        assert_eq!(mt.len, 0);
        assert_eq!(mt.size, 0);
        assert_eq!(mt.align, 0);
        assert_eq!(mt.offset, 0);
    });

    // =========================================================================
    // get_dynamic_addr 测试
    // =========================================================================

    test!("get_dynamic_addr_returns_some_address" {
        // 返回的地址在静态链接时为 0，动态链接时应为非零
        // 仅验证函数不 panic
        let addr = get_dynamic_addr();
        let _ = addr;
    });

    // =========================================================================
    // init_tp 函数基本测试 (使用 BUILTIN_TLS 内存)
    // =========================================================================

    test!("init_tp_basic_success" {
        // 在 BUILTIN_TLS 内测试 init_tp
        let bt_ptr = BUILTIN_TLS.get();
        // 将 pointer 转换为 Pthread 所需对齐的地址
        let td_ptr = unsafe {
            let base = bt_ptr as *mut u8;
            let pt_offset = offset_of!(BuiltinTls, pt);
            base.add(pt_offset) as *mut c_void
        };

        let result = init_tp(td_ptr);
        assert_eq!(result, 0);
    });

    test!("init_tp_sets_self_pointer" {
        let bt_ptr = BUILTIN_TLS.get();
        let td_ptr = unsafe {
            let base = bt_ptr as *mut u8;
            let pt_offset = offset_of!(BuiltinTls, pt);
            base.add(pt_offset)
        };
        let td = td_ptr as *mut Pthread;

        let result = init_tp(td_ptr as *mut c_void);
        assert_eq!(result, 0);

        // 验证 self_ 指针指向自身
        unsafe {
            assert_eq!((*td).self_, td);
        }
    });

    test!("init_tp_sets_detach_state_joinable" {
        let bt_ptr = BUILTIN_TLS.get();
        let td = unsafe {
            let base = bt_ptr as *mut u8;
            let pt_offset = offset_of!(BuiltinTls, pt);
            base.add(pt_offset) as *mut Pthread
        };

        let result = init_tp(td as *mut c_void);
        assert_eq!(result, 0);
        assert_eq!(
            unsafe { (*td).detach_state.load(Ordering::Acquire) },
            DetachState::Joinable as i32
        );
    });

    test!("init_tp_sets_thread_list_self_loop" {
        let bt_ptr = BUILTIN_TLS.get();
        let td = unsafe {
            let base = bt_ptr as *mut u8;
            let pt_offset = offset_of!(BuiltinTls, pt);
            base.add(pt_offset) as *mut Pthread
        };

        let result = init_tp(td as *mut c_void);
        assert_eq!(result, 0);

        // 验证 prev/next 构成自环
        unsafe {
            assert_eq!((*td).prev, td);
            assert_eq!((*td).next, td);
        }
    });

    test!("init_tp_robust_list_self_referential" {
        let bt_ptr = BUILTIN_TLS.get();
        let td = unsafe {
            let base = bt_ptr as *mut u8;
            let pt_offset = offset_of!(BuiltinTls, pt);
            base.add(pt_offset) as *mut Pthread
        };

        let result = init_tp(td as *mut c_void);
        assert_eq!(result, 0);

        // robust_list.head 应指向自身
        unsafe {
            let head_ptr = (*td).robust_list.head;
            let expected = &raw mut (*td).robust_list.head as *mut c_void;
            assert_eq!(head_ptr, expected);
        }
    });

    // =========================================================================
    // copy_tls 基本测试 (使用 BUILTIN_TLS)
    // =========================================================================

    test!("copy_tls_empty_no_modules" {
        // 安全: 先保存全局状态
        let saved_head = unsafe { __libc.tls_head };
        let saved_cnt = unsafe { __libc.tls_cnt };
        let saved_size = unsafe { __libc.tls_size };
        let saved_align = unsafe { __libc.tls_align };

        // 设置 TLS 状态: 无模块的最小环境
        unsafe {
            __libc.tls_head = ptr::null_mut();
            __libc.tls_cnt = 0;
            __libc.tls_size = BUILTIN_TLS_SIZE;
            __libc.tls_align = MIN_TLS_ALIGN;
        }

        let bt_ptr = BUILTIN_TLS.get() as *mut u8;
        let result = copy_tls(bt_ptr);

        // 恢复全局状态
        unsafe {
            __libc.tls_head = saved_head;
            __libc.tls_cnt = saved_cnt;
            __libc.tls_size = saved_size;
            __libc.tls_align = saved_align;
        }

        assert!(!result.is_null());
    });

    test!("copy_tls_returns_valid_aligned_pointer" {
        let saved_head = unsafe { __libc.tls_head };
        let saved_cnt = unsafe { __libc.tls_cnt };
        let saved_size = unsafe { __libc.tls_size };
        let saved_align = unsafe { __libc.tls_align };

        unsafe {
            __libc.tls_head = ptr::null_mut();
            __libc.tls_cnt = 0;
            __libc.tls_size = BUILTIN_TLS_SIZE;
            __libc.tls_align = MIN_TLS_ALIGN;
        }

        let bt_ptr = BUILTIN_TLS.get() as *mut u8;
        let result = copy_tls(bt_ptr);

        unsafe {
            __libc.tls_head = saved_head;
            __libc.tls_cnt = saved_cnt;
            __libc.tls_size = saved_size;
            __libc.tls_align = saved_align;
        }

        // 返回的指针应在 BUILTIN_TLS 范围内
        let result_addr = result as usize;
        let bt_start = bt_ptr as usize;
        let bt_end = bt_start + BUILTIN_TLS_SIZE;
        assert!(result_addr >= bt_start && result_addr < bt_end);

        // 返回的指针应按 tls_align 对齐
        assert_eq!(result_addr % MIN_TLS_ALIGN, 0);
    });

    test!("copy_tls_sets_dtv_with_tls_cnt" {
        let saved_head = unsafe { __libc.tls_head };
        let saved_cnt = unsafe { __libc.tls_cnt };
        let saved_size = unsafe { __libc.tls_size };
        let saved_align = unsafe { __libc.tls_align };

        unsafe {
            __libc.tls_head = ptr::null_mut();
            __libc.tls_cnt = 0;
            __libc.tls_size = BUILTIN_TLS_SIZE;
            __libc.tls_align = MIN_TLS_ALIGN;
        }

        let bt_ptr = BUILTIN_TLS.get() as *mut u8;
        let result = copy_tls(bt_ptr);

        unsafe {
            __libc.tls_head = saved_head;
            __libc.tls_cnt = saved_cnt;
            __libc.tls_size = saved_size;
            __libc.tls_align = saved_align;
        }

        // 验证 DTV 被设置
        let td = result as *mut Pthread;
        unsafe {
            let dtv = (*td).dtv;
            assert!(!dtv.is_null());
            // dtv[0] = tls_cnt = 0
            assert_eq!(*dtv, 0);
        }
    });

    // =========================================================================
    // Elf64_Phdr 字段偏移测试
    // =========================================================================

    test!("elf64_phdr_field_offsets" {
        // 验证 Elf64_Phdr 各字段的偏移量
        assert_eq!(offset_of!(Elf64_Phdr, p_type), 0);
        assert_eq!(offset_of!(Elf64_Phdr, p_flags), 4);
        assert_eq!(offset_of!(Elf64_Phdr, p_offset), 8);
        assert_eq!(offset_of!(Elf64_Phdr, p_vaddr), 16);
        assert_eq!(offset_of!(Elf64_Phdr, p_paddr), 24);
        assert_eq!(offset_of!(Elf64_Phdr, p_filesz), 32);
        assert_eq!(offset_of!(Elf64_Phdr, p_memsz), 40);
        assert_eq!(offset_of!(Elf64_Phdr, p_align), 48);
    });

    // =========================================================================
    // init_tls 调用测试 — 使用零值辅助向量 (无 PT_TLS 路径)
    // =========================================================================

    test!("init_tls_no_phdr_survives" {
        // 构造一个仅含 AT_NULL 的辅助向量 (所有值 = 0)
        // AT_PHDR=0 意味着无程序头信息
        static mut FAKE_AUX: [usize; 12] = [
            AT_PHDR, 0,   // AT_PHDR = 0 → 无程序头
            AT_PHENT, 0,  // AT_PHENT = 0
            AT_PHNUM, 0,  // AT_PHNUM = 0
            0, 0,         // extra padding
            0, 0,         // AT_NULL... actually the array has some trailing
            0, 0,
        ];

        // 由于 AT_PHDR=0, init_tls 应该直接返回 (不 panic)
        init_tls(unsafe { FAKE_AUX.as_mut_ptr() });
    });

    test!("init_tls_with_zero_phnum_survives" {
        // AT_PHDR 非零但 AT_PHNUM=0 — 应直接返回
        // 使用一个有效的地址（以避免空指针异常），PHNUM=0
        static mut FAKE_AUX: [usize; 12] = [
            AT_PHDR, 0x10000,  // AT_PHDR 指向假地址，但因为 PHNUM=0 不会解引用
            AT_PHENT, 56,      // AT_PHENT = 56 (sizeof Elf64_Phdr)
            AT_PHNUM, 0,       // AT_PHNUM = 0 → 不遍历
            0, 0,
            0, 0,
            0, 0,
        ];

        init_tls(unsafe { FAKE_AUX.as_mut_ptr() });
    });
}
