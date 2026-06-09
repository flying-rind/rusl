//! __reset_tls — TLS 重置功能实现。
//!
//! 对应 musl `src/env/__reset_tls.c`。
//!
//! 将当前线程的所有 TLS（Thread-Local Storage）变量恢复到程序加载时的初始值。
//! 采用"逐模块全量复制 + 尾零填充"策略：遍历全局 TLS 模块链表，将每个模块的
//! 初始数据映像（`.tdata` 段）拷贝回当前线程的对应 TLS 内存区域，已初始化部分
//! 之外的内存区域清零（对应 `.tbss` 零初始语义）。
//!
//! # 使用场景
//!
//! - `fork()` 后子进程的 TLS 重置：子进程继承了父进程的完整地址空间（COW），
//!   但 TLS 变量值必须恢复到初始状态
//! - 定时器信号处理线程的 TLS 重置：信号处理线程复用进程地址空间，
//!   在开始处理前必须重置 TLS
//!
//! # 算法
//!
//! ```text
//! 1. __pthread_self() → 获取当前线程控制块 *mut Pthread
//! 2. (*self).dtv → 获取 DTV 数组指针
//! 3. dtv[0] → 读取模块数量 n
//! 4. 若 n == 0 则直接返回
//! 5. 遍历 libc.tls_head 链表 (i = 1..=n):
//!    a. mem = dtv[i] - DTP_OFFSET → TLS 块真实起始地址
//!    b. copy_nonoverlapping(p.image, mem, p.len) → 恢复 .tdata
//!    c. write_bytes(mem + p.len, 0, p.size - p.len) → 清零 .tbss
//!    d. p = p.next → 前进到下一模块
//! ```

use crate::import::libc::tls_module;

// ---------------------------------------------------------------------------
// 常量
// ---------------------------------------------------------------------------

/// DTP_OFFSET — dtv 指针与 TLS 块起始地址之间的固定偏移量。
///
/// `dtv[i]` 存储的是"已偏置值"：`TLS 块真实起始地址 + DTP_OFFSET`。
/// 在解引用前需减去 `DTP_OFFSET` 以恢复 TLS 块的真实地址。
///
/// | 架构 | DTP_OFFSET |
/// |------|-----------|
/// | x86_64 | 0 |
/// | aarch64 | 0 |
/// | arm | 0 |
/// | riscv64 | 0 |
pub(crate) const DTP_OFFSET: usize = 0;

// ---------------------------------------------------------------------------
// 对外接口
// ---------------------------------------------------------------------------

/// 将当前线程的所有 TLS 变量恢复到程序加载时的初始值。
///
/// 遍历全局 TLS 模块链表，将每个模块的初始数据映像（`.tdata` 段）拷贝回
/// 当前线程的对应 TLS 内存区域，已初始化部分之外的内存区域清零（对应
/// `.tbss` 零初始语义）。
///
/// # 前置条件
///
/// - 当前线程的 TLS 必须已通过 TLS 初始化流程（`init_tls` 或 `copy_tls`）
/// - `dtv` 不为 null 且 `dtv[0]` 已正确设置为 TLS 模块数量
/// - 全局链表 `libc.tls_head` 已构建完毕
/// - 调用时应在单线程环境中（或调用者已保证无并发 TLS 访问）
///
/// # 后置条件
///
/// - 所有 TLS 变量的值等同于程序刚加载时的初始值
/// - 不修改 `libc.tls_head`、`dtv` 指针或任何全局状态
///
/// # 错误处理
///
/// 此函数不返回错误码。若前置条件不满足（如 DTV 未初始化），函数安全地
/// 作为空操作返回。
///
/// [Visibility]: pub(crate) — 仅供 rusl 内部使用，POSIX/C 标准未定义
pub(crate) fn __reset_tls() {
    // Step 1: 获取当前线程控制块指针
    let self_ = crate::import::pthread_impl::__pthread_self();
    if self_.is_null() {
        // 线程尚未初始化，无法执行 TLS 重置
        return;
    }

    // Step 2: 获取 DTV 数组指针
    let dtv = unsafe { (*self_).dtv };
    if dtv.is_null() {
        // DTV 未分配，无法执行 TLS 重置
        return;
    }

    // Step 3: 获取全局 TLS 模块链表头
    let tls_head = unsafe { crate::import::libc::__libc.tls_head };

    // Step 4: 执行核心重置算法
    reset_tls_core(dtv, tls_head, DTP_OFFSET);
}

/// C ABI 导出：__reset_tls —— 供 musl timer_create 调用。
#[export_name = "__reset_tls"]
pub extern "C" fn __reset_tls_c() {
    __reset_tls()
}

// ---------------------------------------------------------------------------
// 核心算法（可单独测试）
// ---------------------------------------------------------------------------

/// TLS 重置的核心算法实现。
///
/// 此函数接收显式参数，便于单元测试。生产代码应通过 [`__reset_tls`] 调用。
///
/// # 参数
///
/// * `dtv` — DTV 数组指针，`dtv[0]` = 模块数量 n，`dtv[i]` = 模块 i 的偏置 TLS 块地址
/// * `tls_head` — 全局 TLS 模块链表头，顺序与 DTV 索引 1..=n 一一对应
/// * `dtp_offset` — DTV 指针偏置值，用于恢复 TLS 块真实地址
///
/// # 算法
///
/// 遍历 `tls_head` 链表（共 n 个模块），对每个模块执行：
/// 1. 从 DTV 获取该模块的 TLS 块偏置地址
/// 2. 减去 `dtp_offset` 得到 TLS 块真实起始地址
/// 3. 将模块初始数据拷贝到 TLS 块（`.tdata` 段恢复）
/// 4. 将剩余区域清零（`.tbss` 段恢复）
fn reset_tls_core(dtv: *mut usize, tls_head: *mut tls_module, dtp_offset: usize) {
    // 读取模块数量 n = dtv[0]
    let n = unsafe { dtv.read() };
    if n == 0 {
        // 无 TLS 模块，无需重置
        return;
    }

    let mut p: *mut tls_module = tls_head;
    let mut i: usize = 1;

    while i <= n {
        // 防御：模块链表耗尽
        if p.is_null() {
            break;
        }

        unsafe {
            // 读取当前模块的 TLS 块偏置地址
            let dtv_val = dtv.add(i).read();
            debug_assert!(
                dtv_val >= dtp_offset,
                "dtv[{}] = {:#x} < DTP_OFFSET = {:#x}",
                i,
                dtv_val,
                dtp_offset
            );
            let mem = dtv_val.wrapping_sub(dtp_offset) as *mut u8;

            // 读取模块描述符字段
            let img = (*p).image;
            let len = (*p).len;
            let size = (*p).size;
            let next = (*p).next;

            // 拷贝已初始化数据：将 .tdata 段从初始映像复制到 TLS 块
            if len > 0 {
                core::ptr::copy_nonoverlapping(img as *const u8, mem, len);
            }

            // 清零 .tbss 区域：已初始化数据之后的部分全部置零
            // 仅当 len < size 时才清零，防御 len > size 的不变量违规
            if len < size {
                core::ptr::write_bytes(mem.add(len), 0u8, size - len);
            }

            // 前进到下一模块
            p = next;
        }

        i += 1;
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::*;
    use crate::import::pthread_impl::Pthread;
    use core::ffi::c_void;

    // =========================================================================
    // 常量测试
    // =========================================================================

    test!("dtp_offset_is_zero" {
        assert_eq!(DTP_OFFSET, 0);
    });

    test!("dtp_offset_fits_in_usize" {
        let _: usize = DTP_OFFSET;
    });

    // =========================================================================
    // 空操作路径测试：n == 0（无 TLS 模块）
    // =========================================================================

    test!("reset_tls_core_n_zero_returns_immediately" {
        // 构造一个 DTV 数组，dtv[0] = 0 表示无模块
        let mut dtv_buf: [usize; 1] = [0];
        let dtv_ptr: *mut usize = dtv_buf.as_mut_ptr();

        // 设置一个非空的 tls_head —— 但 n=0 意味着不会被访问
        let mut module = tls_module {
            next: core::ptr::null_mut(),
            image: core::ptr::null_mut(),
            len: 42,
            size: 100,
            align: 0,
            offset: 0,
        };
        let tls_head: *mut tls_module = &mut module;

        // 调用后不应崩溃，且不应访问模块
        reset_tls_core(dtv_ptr, tls_head, 0);

        // 模块数据未被修改（验证 n=0 时确实未进入循环）
        assert_eq!(module.len, 42);
        assert_eq!(module.size, 100);
    });

    test!("reset_tls_core_n_zero_with_null_head" {
        let mut dtv_buf: [usize; 1] = [0];
        let dtv_ptr: *mut usize = dtv_buf.as_mut_ptr();

        // tls_head 为 null，n=0 时不应解引用
        reset_tls_core(dtv_ptr, core::ptr::null_mut(), 0);
    });

    // =========================================================================
    // 单元模块重置测试：单一模块（.tdata 拷贝 + .tbss 清零）
    // =========================================================================

    test!("reset_tls_core_single_module_with_tbss" {
        // 目标缓冲区：8 字节已初始化数据 + 8 字节 .tbss
        let tls_buf: [u8; 16] = [0xABu8; 16];

        // 初始映像：8 字节的已知模式
        let image_data: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        // DTV 数组: dtv[0] = 1, dtv[1] = tls_buf 地址
        let mut dtv_buf: [usize; 2] = [1, tls_buf.as_ptr() as usize];

        // TLS 模块描述符
        let mut module = tls_module {
            next: core::ptr::null_mut(),
            image: image_data.as_ptr() as *mut c_void,
            len: 8,   // .tdata 大小
            size: 16, // 总大小（.tdata + .tbss）
            align: 8,
            offset: 0,
        };

        let dtv_ptr: *mut usize = dtv_buf.as_mut_ptr();
        let tls_head: *mut tls_module = &mut module;

        reset_tls_core(dtv_ptr, tls_head, 0);

        // 验证 .tdata 部分被正确拷贝
        for k in 0..8 {
            assert_eq!(
                tls_buf[k], image_data[k],
                "tls_buf[{}] = {:#x}, expected {:#x}",
                k, tls_buf[k], image_data[k]
            );
        }

        // 验证 .tbss 部分被清零
        for k in 8..16 {
            assert_eq!(tls_buf[k], 0u8, "tls_buf[{}] should be 0, got {:#x}", k, tls_buf[k]);
        }
    });

    test!("reset_tls_core_single_module_no_tbss" {
        // 当 len == size 时，没有 .tbss 区域
        let tls_buf: [u8; 8] = [0xFFu8; 8];
        let image_data: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22];

        let mut dtv_buf: [usize; 2] = [1, tls_buf.as_ptr() as usize];

        let mut module = tls_module {
            next: core::ptr::null_mut(),
            image: image_data.as_ptr() as *mut c_void,
            len: 8,
            size: 8, // len == size，无 .tbss
            align: 8,
            offset: 0,
        };

        let dtv_ptr: *mut usize = dtv_buf.as_mut_ptr();
        let tls_head: *mut tls_module = &mut module;

        reset_tls_core(dtv_ptr, tls_head, 0);

        // 所有字节应与源数据一致
        for k in 0..8 {
            assert_eq!(tls_buf[k], image_data[k], "tls_buf[{}] mismatch", k);
        }
    });

    test!("reset_tls_core_single_module_only_tbss" {
        // 当 len == 0 时，只有 .tbss（全部清零）
        let tls_buf: [u8; 8] = [0xFFu8; 8];

        let mut dtv_buf: [usize; 2] = [1, tls_buf.as_ptr() as usize];

        let mut module = tls_module {
            next: core::ptr::null_mut(),
            image: core::ptr::null_mut(),
            len: 0,
            size: 8,
            align: 8,
            offset: 0,
        };

        let dtv_ptr: *mut usize = dtv_buf.as_mut_ptr();
        let tls_head: *mut tls_module = &mut module;

        reset_tls_core(dtv_ptr, tls_head, 0);

        // 全部应为 0
        for k in 0..8 {
            assert_eq!(tls_buf[k], 0u8, "tls_buf[{}] should be 0", k);
        }
    });

    // =========================================================================
    // 多模块测试
    // =========================================================================

    test!("reset_tls_core_two_modules" {
        // 模块 1：4 字节 .tdata + 4 字节 .tbss
        let tls1: [u8; 8] = [0xFFu8; 8];
        let img1: [u8; 4] = [0x10, 0x20, 0x30, 0x40];

        // 模块 2：6 字节 .tdata + 2 字节 .tbss
        let tls2: [u8; 8] = [0xFFu8; 8];
        let img2: [u8; 6] = [0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6];

        // DTV: [2, tls1_addr, tls2_addr]
        let mut dtv_buf: [usize; 3] = [
            2,
            tls1.as_ptr() as usize,
            tls2.as_ptr() as usize,
        ];

        // 模块链表
        let mut mod2 = tls_module {
            next: core::ptr::null_mut(),
            image: img2.as_ptr() as *mut c_void,
            len: 6,
            size: 8,
            align: 8,
            offset: 0,
        };
        let mut mod1 = tls_module {
            next: &mut mod2,
            image: img1.as_ptr() as *mut c_void,
            len: 4,
            size: 8,
            align: 8,
            offset: 0,
        };

        reset_tls_core(dtv_buf.as_mut_ptr(), &mut mod1, 0);

        // 验证模块 1
        for k in 0..4 {
            assert_eq!(tls1[k], img1[k], "tls1[{}] mismatch", k);
        }
        for k in 4..8 {
            assert_eq!(tls1[k], 0u8, "tls1[{}] should be 0", k);
        }

        // 验证模块 2
        for k in 0..6 {
            assert_eq!(tls2[k], img2[k], "tls2[{}] mismatch", k);
        }
        for k in 6..8 {
            assert_eq!(tls2[k], 0u8, "tls2[{}] should be 0", k);
        }
    });

    test!("reset_tls_core_n_exceeds_chain_length" {
        // n=3 但链表只有 2 个模块 —— 应在遇到 null 时安全退出
        let tls1: [u8; 4] = [0xFFu8; 4];
        let img1: [u8; 4] = [0x01, 0x02, 0x03, 0x04];
        let tls2: [u8; 4] = [0xFFu8; 4];
        let img2: [u8; 4] = [0x05, 0x06, 0x07, 0x08];

        // DTV: [3, tls1, tls2, unused_addr]
        let mut dtv_buf: [usize; 4] = [
            3,
            tls1.as_ptr() as usize,
            tls2.as_ptr() as usize,
            0xDEAD_BEEF,
        ];

        // 链表只有 2 个模块（mod2.next == null）
        let mut mod2 = tls_module {
            next: core::ptr::null_mut(),
            image: img2.as_ptr() as *mut c_void,
            len: 4,
            size: 4,
            align: 4,
            offset: 0,
        };
        let mut mod1 = tls_module {
            next: &mut mod2,
            image: img1.as_ptr() as *mut c_void,
            len: 4,
            size: 4,
            align: 4,
            offset: 0,
        };

        // 不应崩溃
        reset_tls_core(dtv_buf.as_mut_ptr(), &mut mod1, 0);

        // 前两个模块被正确重置
        for k in 0..4 {
            assert_eq!(tls1[k], img1[k]);
            assert_eq!(tls2[k], img2[k]);
        }
    });

    // =========================================================================
    // DTP_OFFSET 非零测试
    // =========================================================================

    test!("reset_tls_core_nonzero_dtp_offset" {
        // 模拟 DTP_OFFSET = 0x1000 的场景
        let tls_buf: [u8; 8] = [0xFFu8; 8];
        let image_data: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBE];

        // DTV 中存储偏置后的地址
        let biased_addr = (tls_buf.as_ptr() as usize) + 0x1000;
        let mut dtv_buf: [usize; 2] = [1, biased_addr];

        let mut module = tls_module {
            next: core::ptr::null_mut(),
            image: image_data.as_ptr() as *mut c_void,
            len: 4,
            size: 8,
            align: 4,
            offset: 0,
        };

        reset_tls_core(dtv_buf.as_mut_ptr(), &mut module, 0x1000);

        // 验证数据正确拷贝到了正确的地址
        for k in 0..4 {
            assert_eq!(tls_buf[k], image_data[k], "tls_buf[{}] mismatch", k);
        }
        // .tbss 部分清零
        for k in 4..8 {
            assert_eq!(tls_buf[k], 0u8, "tls_buf[{}] should be 0", k);
        }
    });

    test!("reset_tls_core_zero_dtp_offset" {
        let tls_buf: [u8; 4] = [0xFFu8; 4];
        let image_data: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];

        let mut dtv_buf: [usize; 2] = [1, tls_buf.as_ptr() as usize];

        let mut module = tls_module {
            next: core::ptr::null_mut(),
            image: image_data.as_ptr() as *mut c_void,
            len: 4,
            size: 4,
            align: 4,
            offset: 0,
        };

        // DTP_OFFSET = 0 时，dtv[i] 就是真实地址
        reset_tls_core(dtv_buf.as_mut_ptr(), &mut module, 0);

        for k in 0..4 {
            assert_eq!(tls_buf[k], image_data[k]);
        }
    });

    // =========================================================================
    // 边界条件测试
    // =========================================================================

    test!("reset_tls_core_len_gt_size_protection" {
        // 当 len > size 时（违反不变量），tbss 的 wrapping_sub 会回绕
        // 但 write_bytes 的大小为 0 时会跳过，而 copy 仍执行 len 字节
        // 此测试验证不会崩溃
        let tls_buf: [u8; 16] = [0xFFu8; 16];
        let image_data: [u8; 8] = [0x01; 8];

        let mut dtv_buf: [usize; 2] = [1, tls_buf.as_ptr() as usize];

        let mut module = tls_module {
            next: core::ptr::null_mut(),
            image: image_data.as_ptr() as *mut c_void,
            len: 8,
            size: 4, // 异常: size < len
            align: 4,
            offset: 0,
        };

        // 不应崩溃 —— wrapping_sub 会回绕产生巨大值，但 write_bytes 会执行
        reset_tls_core(dtv_buf.as_mut_ptr(), &mut module, 0);
    });

    // =========================================================================
    // __reset_tls 公开接口测试（空操作路径）
    // =========================================================================

    test!("__reset_tls_does_not_crash" {
        // 在单元测试环境中，TLS 可能未初始化，__pthread_self() 可能返回
        // null 或未设置 DTV 的指针。但函数应安全地作为空操作返回。
        __reset_tls();
    });

    test!("__reset_tls_callable_twice" {
        // 连续两次调用不应崩溃
        __reset_tls();
        __reset_tls();
    });

    test!("__reset_tls_callable_multiple_times" {
        for _ in 0..5 {
            __reset_tls();
        }
    });
}
