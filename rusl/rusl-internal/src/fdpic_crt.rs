//! fdpic_crt 模块 — FDPIC ELF 程序 C 运行时启动修复。
//!
//! 本模块实现了 `__fdpic_fixup` 函数，用于修复 FDPIC 程序中的
//! 函数描述符和 GOT 指针。FDPIC（Function Descriptor Position-Independent
//! Code）用于无 MMU 的嵌入式 Linux 系统。
//!
//! # 编译条件
//!
//! 仅在 FDPIC 目标平台编译。标准 x86_64 等有 MMU 的平台不需要。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

use core::ffi::c_void;

// ---------------------------------------------------------------------------
// 内部数据结构
// ---------------------------------------------------------------------------

/// FDPIC 加载段描述符（仅在 `__fdpic_fixup` 内部使用）。
///
/// 每个段描述一个 ELF 加载段的虚拟地址与实际加载地址之间的映射关系。
#[repr(C)]
struct FdpicLoadseg {
    /// 实际加载地址（内核将段映射到的物理/实际位置）
    addr: usize,
    /// 虚拟地址（ELF p_vaddr，即段期望的虚拟起始地址）
    p_vaddr: usize,
    /// 内存大小（ELF p_memsz）
    p_memsz: usize,
}

/// FDPIC 加载映射表头（仅在 `__fdpic_fixup` 内部使用）。
///
/// 由 FDPIC 内核加载器在程序启动时传递给进程。
/// 包含段数量和段描述符数组（灵活数组成员）。
#[repr(C)]
struct FdpicLoadmap {
    /// 版本号
    version: u16,
    /// 段数量
    nsegs: u16,
    // segs: FdpicLoadseg[] — 灵活数组成员，通过指针运算访问
}

// ---------------------------------------------------------------------------
// 公共接口
// ---------------------------------------------------------------------------

/// 执行 FDPIC 程序的函数描述符修复。
///
/// 在程序启动时，将由内核传递的加载映射表信息用于修正
/// `.got` 段中的 GOT 指针和函数描述符中的代码地址。
///
/// # 参数
///
/// * `map` - FDPIC 加载映射表指针（可为 null，表示非 FDPIC 加载器）
/// * `a` - 待修复地址数组的起始位置
/// * `z` - 待修复地址数组的结束位置
///
/// # 返回值
///
/// 修正后的 GOT 指针
///
/// # Safety
///
/// * `a` 和 `z` 必须指向有效的待修复地址数组
/// * `a < z`
/// * 若 `map` 非空，其内部结构必须符合 FDPIC 加载映射表格式
///
/// # 系统算法
///
/// 遍历地址数组 `[a, z)`，对每个地址执行两次定位：
/// 1. 找到该地址属于哪个实际加载段，计算修正后的地址
/// 2. 找到修正后地址指向的值属于哪个虚拟加载段，再次应用偏移
///
/// 算法在 seg[] 数组上线性搜索（而非二分搜索），因为典型的
/// FDPIC 程序只有 2-3 个段，线性扫描足够高效。
///
/// **Key Insight**: 当搜索超出 nsegs 时回绕到 0（处理段数组不
/// 覆盖整个地址空间的情况）。
///
/// 当前标准 x86_64/aarch64 等有 MMU 的平台不需要此函数。
/// 仅在 FDPIC 目标平台（如 ARM no-MMU）上编译。
pub unsafe fn __fdpic_fixup(
    map: *const c_void,
    a: *mut usize,
    z: *const usize,
) -> *mut c_void {
    // Case 1: map == null —— 程序由非 FDPIC ELF 加载器加载。
    // 不需要修复，直接返回 z[-1] 作为 GOT 指针。
    //
    // 这与 C 实现一致：
    //   if (!map) return (void *)z[-1];
    if map.is_null() {
        // Safety: a < z 是前置条件，因此 z 至少指向 a + 1 的位置。
        // z.sub(1) 指向有效的 usize 地址。
        let got_ptr = unsafe { z.sub(1).read() };
        return got_ptr as *mut c_void;
    }

    // Case 2: map != null —— 执行完整的 FDPIC 修复算法。
    //
    // 解析加载映射表：
    //   映射表在内存中的布局为 [FdpicLoadmap header] [FdpicLoadseg; nsegs]
    //
    // 我们使用原始指针运算来访问灵活数组成员 segs[]。
    let lm = unsafe { &*(map as *const FdpicLoadmap) };
    let nsegs = lm.nsegs as usize;

    // Safety: segs[] 紧跟在 FdpicLoadmap header 之后。
    // 使用指针运算计算 segs 数组的起始地址。
    let segs_ptr = unsafe { (map as *const u8).add(core::mem::size_of::<FdpicLoadmap>()) }
        as *const FdpicLoadseg;

    // 将 segs 视为切片以便安全索引访问。
    // Safety: 调用者保证 map 指向的内存符合 FdpicLoadmap 格式，
    // 且 segs[] 数组包含至少 nsegs 个有效条目。
    let segs = unsafe { core::slice::from_raw_parts(segs_ptr, nsegs) };

    let mut rseg: usize = 0;
    let mut vseg: usize = 0;
    let mut a = a;

    loop {
        // 步骤 1: 定位当前待修复地址 *a 所属的"实际"加载段 (rseg)。
        //
        // 在 segs[] 中线性搜索，找到满足以下条件的段 r：
        //   *a - segs[r].p_vaddr < segs[r].p_memsz
        //
        // 即 *a 的虚拟地址偏移落入段 r 的虚拟地址范围。
        //
        // 若超出 nsegs，回绕到 0 — 这处理了跨段边界情况。
        while unsafe { *a.wrapping_sub(segs[rseg].p_vaddr) } >= segs[rseg].p_memsz {
            rseg += 1;
            if rseg == nsegs {
                rseg = 0;
            }
        }

        // 步骤 2: 计算修正后的地址 r。
        //
        // r = *a + addr - p_vaddr
        // 即将虚拟地址偏移应用于实际加载地址。
        //
        // 用 wrapping_add / wrapping_sub 避免中间值溢出产生未定义行为。
        let r_addr = unsafe {
            a.read().wrapping_add(segs[rseg].addr).wrapping_sub(segs[rseg].p_vaddr)
        };
        let r = r_addr as *mut usize;

        // 步骤 3: 递增 a；若已处理完所有条目，返回最后一个修正后的地址
        //        （即 GOT 指针）。
        a = unsafe { a.add(1) };
        if a as *const usize == z {
            return r as *mut c_void;
        }

        // 步骤 4: 定位修正后地址 *r 所指向的值属于哪个"虚拟"加载段 (vseg)。
        //
        // 这处理了跨段引用 — 当 GOT 条目或函数描述符指向的地址位于
        // 不同段时，需要再次应用偏移。
        //
        // 在 segs[] 中线性搜索，满足：
        //   *r - segs[v].p_vaddr < segs[v].p_memsz
        while unsafe { r.read().wrapping_sub(segs[vseg].p_vaddr) } >= segs[vseg].p_memsz {
            vseg += 1;
            if vseg == nsegs {
                vseg = 0;
            }
        }

        // 步骤 5: 修正 *r — 将 GOT/描述符中的虚拟地址转换为实际地址。
        //
        // *r += addr - p_vaddr
        // 即加上该段实际加载地址相对虚拟地址的偏移量。
        unsafe {
            r.write(
                r.read()
                    .wrapping_add(segs[vseg].addr)
                    .wrapping_sub(segs[vseg].p_vaddr),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use rusl_core::test;

    // 测试 null map 路径：返回 z[-1] 作为 GOT 指针。
    //
    // 模拟非 FDPIC 加载器场景：map 为空，直接返回数组最后一个元素。
    test!("fdpic_fixup_null_map" {
        let mut arr: [usize; 3] = [0x1000, 0x2000, 0xDEAD_BEEF];
        let a = arr.as_mut_ptr();
        let z = unsafe { a.add(3) };

        let result = unsafe {
            super::__fdpic_fixup(core::ptr::null(), a, z)
        };

        // null map 时应返回 z[-1]（即 arr[2] 的值）
        assert_eq!(result as usize, 0xDEAD_BEEF);

        // 数组内容不应被修改
        assert_eq!(arr[0], 0x1000);
        assert_eq!(arr[1], 0x2000);
        assert_eq!(arr[2], 0xDEAD_BEEF);
    });

    // 测试 null map 路径：单元素数组。
    test!("fdpic_fixup_null_map_single" {
        let mut arr: [usize; 1] = [0xCAFE_BABE];
        let a = arr.as_mut_ptr();
        let z = unsafe { a.add(1) };

        let result = unsafe {
            super::__fdpic_fixup(core::ptr::null(), a, z)
        };

        assert_eq!(result as usize, 0xCAFE_BABE);
    });

    // 验证 FdpicLoadseg 结构体大小与 C 实现一致。
    //
    // C 中结构体为：
    //   struct fdpic_loadseg {
    //       uintptr_t addr, p_vaddr, p_memsz;
    //   };
    //
    // uintptr_t 在 64 位系统上为 8 字节，3 个字段 = 24 字节。
    test!("fdpic_loadseg_size" {
        use core::mem::size_of;
        assert_eq!(size_of::<super::FdpicLoadseg>(), 24);
    });

    // 验证 FdpicLoadmap 结构体大小与 C 实现一致。
    //
    // C 中结构体为：
    //   struct fdpic_loadmap {
    //       unsigned short version, nsegs;  // 2 * 2 = 4 字节
    //       // segs[] 柔性数组不计入 sizeof
    //   };
    test!("fdpic_loadmap_size" {
        use core::mem::size_of;
        assert_eq!(size_of::<super::FdpicLoadmap>(), 4);
    });

    // 验证 FdpicLoadmap 和 FdpicLoadseg 满足 #[repr(C)] 对齐要求。
    test!("fdpic_structs_alignment" {
        use core::mem::align_of;
        // FdpicLoadmap 包含 u16 字段，对齐为 2
        assert!(align_of::<super::FdpicLoadmap>() >= 2);
        // FdpicLoadseg 包含 usize 字段，对齐为 8（64位）
        assert!(align_of::<super::FdpicLoadseg>() >= 8);
    });
}