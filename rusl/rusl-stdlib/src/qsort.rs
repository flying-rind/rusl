//! qsort/qsort_r/__qsort_r —— 通用数组排序（Smoothsort 算法）。对外导出 C ABI 兼容的符号。
//!
//! 算法: Smoothsort（自适应 Heapsort），最坏情况 O(n log n)，近似有序时 O(n)。
//! 代码是 musl 对应 C 实现 (src/stdlib/qsort.c) 的忠实 Rust 翻译。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;

/// qsort 比较函数类型：`cmp(a, b)` 返回 <0、=0 或 >0。
pub type CmpFun = unsafe extern "C" fn(*const c_void, *const c_void) -> i32;

/// qsort_r 比较函数类型：`cmp(a, b, arg)` 返回 <0、=0 或 >0。
pub type CmpFunR = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> i32;

// ========== 常量 ==========

/// `size_t`（即 `usize`）的位数，64 位平台为 64。
const WORD_BITS: usize = core::mem::size_of::<usize>() * 8;

/// 工作数组长度：16 × sizeof(size_t)。
const AR_LEN: usize = 16 * core::mem::size_of::<usize>();

/// 工作数组掩码。
const AR_MASK: usize = AR_LEN - 1;

/// 莱昂纳多数数组长度：12 × sizeof(size_t)。
const LP_LEN: usize = 12 * core::mem::size_of::<usize>();

// ========== 内部辅助函数 ==========

/// 返回双字 `p[0..1]` 中从第 1 位（不含第 0 位）开始的第一个置位位的位置。
unsafe fn pntz(p: &[usize; 2]) -> i32 {
    if p[0] != 1 {
        return (p[0] - 1).trailing_zeros() as i32;
    }
    if p[1] != 0 {
        (WORD_BITS as i32) + p[1].trailing_zeros() as i32
    } else {
        0
    }
}

/// 循环左移双字 `p`，移动 `n` 位。
unsafe fn shl(p: &mut [usize; 2], n: i32) {
    let mut n = n as usize;
    if n == 0 {
        return;
    }
    if n >= WORD_BITS {
        n -= WORD_BITS;
        p[1] = p[0];
        p[0] = 0;
        if n == 0 {
            return;
        }
    }
    p[1] = (p[1] << n) | (p[0] >> (WORD_BITS - n));
    p[0] <<= n;
}

/// 循环右移双字 `p`，移动 `n` 位。
unsafe fn shr(p: &mut [usize; 2], n: i32) {
    let mut n = n as usize;
    if n == 0 {
        return;
    }
    if n >= WORD_BITS {
        n -= WORD_BITS;
        p[0] = p[1];
        p[1] = 0;
        if n == 0 {
            return;
        }
    }
    p[0] = (p[0] >> n) | (p[1] << (WORD_BITS - n));
    p[1] >>= n;
}

/// 旋转 `n` 个指针：`ar[0] ← ar[1] ← ... ← ar[n-1] ← ar[0]`（循环左移一个位置）。
unsafe fn cycle(width: usize, ar: &mut [*mut u8], n: usize) {
    if n < 2 {
        return;
    }
    let mut tmp: [u8; 256] = [0; 256];
    ar[n] = tmp.as_mut_ptr();
    let mut w = width;
    while w > 0 {
        let l = if 256 < w { 256 } else { w };
        // tmp ← ar[0]
        core::ptr::copy_nonoverlapping(ar[0] as *const u8, ar[n], l);
        for i in 0..n {
            core::ptr::copy_nonoverlapping(ar[i + 1] as *const u8, ar[i], l);
            ar[i] = ar[i].add(l);
        }
        w -= l;
    }
}

/// sift —— 在莱昂纳多堆中做下沉操作。
unsafe fn sift(
    head: *mut u8,
    width: usize,
    cmp: CmpFunR,
    arg: *mut c_void,
    pshift: i32,
    lp: &[usize],
) {
    let mut ar: [*mut u8; AR_LEN] = [core::ptr::null_mut(); AR_LEN];
    let mut pshift = pshift;
    let mut head = head;
    let mut i = 1usize;
    ar[0] = head;

    while pshift > 1 {
        let rt = head.sub(width);
        let lf = head.sub(width + lp[pshift as usize - 2]);

        if cmp(ar[0] as *const c_void, lf as *const c_void, arg) >= 0
            && cmp(ar[0] as *const c_void, rt as *const c_void, arg) >= 0
        {
            break;
        }
        if cmp(lf as *const c_void, rt as *const c_void, arg) >= 0 {
            ar[i & AR_MASK] = lf;
            i += 1;
            head = lf;
            pshift -= 1;
        } else {
            ar[i & AR_MASK] = rt;
            i += 1;
            head = rt;
            pshift -= 2;
        }
    }
    cycle(width, &mut ar, i & AR_MASK);
}

/// trinkle —— 在莱昂纳多堆中做"整理"操作。
unsafe fn trinkle(
    head: *mut u8,
    width: usize,
    cmp: CmpFunR,
    arg: *mut c_void,
    pp: &[usize; 2],
    pshift: i32,
    trusty: i32,
    lp: &[usize],
) {
    // 拷贝 pp 到局部 p（不修改原值）
    let mut p = [pp[0], pp[1]];
    let mut ar: [*mut u8; AR_LEN] = [core::ptr::null_mut(); AR_LEN];
    let mut pshift = pshift;
    let mut head = head;
    let mut trusty = trusty;
    let mut i = 1usize;
    ar[0] = head;

    while p[0] != 1 || p[1] != 0 {
        let stepson = head.sub(lp[pshift as usize]);
        if cmp(stepson as *const c_void, ar[0] as *const c_void, arg) <= 0 {
            break;
        }
        if trusty == 0 && pshift > 1 {
            let rt = head.sub(width);
            let lf = head.sub(width + lp[pshift as usize - 2]);
            if cmp(rt as *const c_void, stepson as *const c_void, arg) >= 0
                || cmp(lf as *const c_void, stepson as *const c_void, arg) >= 0
            {
                break;
            }
        }

        ar[i & AR_MASK] = stepson;
        i += 1;
        head = stepson;
        let trail = pntz(&p);
        shr(&mut p, trail);
        pshift += trail;
        trusty = 0;
    }
    if trusty == 0 {
        cycle(width, &mut ar, i & AR_MASK);
        sift(head, width, cmp, arg, pshift, lp);
    }
}

// ========== 公开 API ==========

/// qsort 的 POSIX 扩展版本（`__qsort_r`），透传 `arg` 给比较函数。
///
/// 入口函数，仅做参数校验。`nel <= 1` 时直接返回。
///
/// # Safety
///
/// - `base` 必须指向至少 `nel * width` 字节的可读写内存。
/// - `cmp` 是比较函数，不得修改数组元素。
/// - `arg` 透传给比较函数。
#[no_mangle]
pub unsafe extern "C" fn __qsort_r(
    base: *mut c_void,
    nel: usize,
    width: usize,
    cmp: Option<CmpFunR>,
    arg: *mut c_void,
) {
    let cmp = match cmp {
        Some(f) => f,
        None => return,
    };

    let size = width.wrapping_mul(nel);
    if size == 0 {
        return;
    }

    let head = base as *mut u8;
    let high = head.add(size - width);

    // 预计算莱昂纳多数 L(i)，按 width 缩放
    let mut lp = [0usize; LP_LEN];
    lp[0] = width;
    lp[1] = width;
    let mut i = 2;
    while i < LP_LEN {
        let val = lp[i - 2].wrapping_add(lp[i - 1]).wrapping_add(width);
        lp[i] = val;
        if val >= size {
            break;
        }
        i += 1;
    }

    let mut p = [1usize, 0];
    let mut pshift = 1i32;
    let mut head = head;

    // 阶段 1 —— 建堆
    while head < high {
        if p[0] & 3 == 3 {
            sift(head, width, cmp, arg, pshift, &lp);
            shr(&mut p, 2);
            pshift += 2;
        } else {
            let remaining = (high as usize) - (head as usize);
            if lp[pshift as usize - 1] >= remaining {
                trinkle(head, width, cmp, arg, &p, pshift, 0, &lp);
            } else {
                sift(head, width, cmp, arg, pshift, &lp);
            }

            if pshift == 1 {
                shl(&mut p, 1);
                pshift = 0;
            } else {
                shl(&mut p, pshift - 1);
                pshift = 1;
            }
        }

        p[0] |= 1;
        head = head.add(width);
    }

    // 阶段 2 —— 拆堆
    trinkle(head, width, cmp, arg, &p, pshift, 0, &lp);

    while pshift != 1 || p[0] != 1 || p[1] != 0 {
        if pshift <= 1 {
            let trail = pntz(&p);
            shr(&mut p, trail);
            pshift += trail;
        } else {
            shl(&mut p, 2);
            pshift -= 2;
            p[0] ^= 7;
            shr(&mut p, 1);
            trinkle(
                head.sub(lp[pshift as usize]).sub(width),
                width,
                cmp,
                arg,
                &p,
                pshift + 1,
                1,
                &lp,
            );
            shl(&mut p, 1);
            p[0] |= 1;
            trinkle(head.sub(width), width, cmp, arg, &p, pshift, 1, &lp);
        }
        head = head.sub(width);
    }
}

/// `qsort_r` 是 `__qsort_r` 的别名（与 musl 的 weak_alias 行为一致）。
#[no_mangle]
pub extern "C" fn qsort_r(
    base: *mut c_void,
    nel: usize,
    width: usize,
    cmp: Option<CmpFunR>,
    arg: *mut c_void,
) {
    unsafe {
        __qsort_r(base, nel, width, cmp, arg);
    }
}

// ========== qsort 包装 ==========

/// 将 `qsort` 的 2 参数比较器包装为 3 参数比较器。
unsafe extern "C" fn qsort_wrapper(
    a: *const c_void,
    b: *const c_void,
    arg: *mut c_void,
) -> i32 {
    let cmp: CmpFun = core::mem::transmute(arg);
    cmp(a, b)
}

/// 对数组进行原地升序排序（不稳定）。
///
/// 使用 Smoothsort（自适应 Heapsort）算法：
/// - 最坏时间复杂度: O(n log n)
/// - 接近有序时: O(n)
/// - 空间复杂度: O(1)
///
/// # Safety
///
/// - `base` 必须指向至少 `nel * width` 字节的可读写内存。
/// - `cmp` 是比较函数，不得修改数组元素。
/// - 每对元素通过 `cmp` 比较时，传入的指针在其生命周期内有效。
#[no_mangle]
pub extern "C" fn qsort(
    base: *mut c_void,
    nel: usize,
    width: usize,
    cmp: Option<CmpFun>,
) {
    let cmp = match cmp {
        Some(f) => f,
        None => return,
    };
    unsafe {
        __qsort_r(
            base,
            nel,
            width,
            Some(qsort_wrapper),
            core::mem::transmute(cmp),
        );
    }
}
