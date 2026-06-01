//! emulate_wait4 模块 — 用 `SYS_waitid` 模拟 `wait4()` 系统调用。
//!
//! 本模块实现了在缺少原生 `SYS_wait4` 系统调用的平台上，
//! 通过 `SYS_waitid` 来模拟 `wait4()` 的兼容层。
//!
//! # 编译条件
//!
//! 仅在目标架构不提供原生 `wait4` 系统调用时才编译此函数。
//! 当前 x86_64 和 aarch64 等主流架构有原生 `SYS_wait4`，
//! 不需要此模拟。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用，
//! 由 `__sys_wait4` / `__sys_wait4_cp` 宏间接调用。

use core::ffi::{c_int, c_long, c_void};

// ---------------------------------------------------------------------------
// POSIX waitid 相关常量
// 与 Linux 内核 include/uapi/linux/wait.h 保持一致
// ---------------------------------------------------------------------------

/// 等待指定进程 ID 的子进程。
const P_PID: c_int = 1;

/// 等待指定进程组 ID 的子进程。
const P_PGID: c_int = 2;

/// 等待任意子进程。
const P_ALL: c_int = 0;

/// 等待已终止（正常退出或信号杀死）的子进程。
const WEXITED: c_int = 4;

/// 非阻塞：无匹配子进程时立即返回 0 而非阻塞。
#[allow(dead_code)]
const WNOHANG: c_int = 1;

/// 同时等待已停止的子进程。
#[allow(dead_code)]
const WUNTRACED: c_int = 2;

/// musl 内部标志：不丢弃因 ptrace 而停止的子进程。
#[allow(dead_code)]
const __WALL: c_int = 0x40000000;

// ---------------------------------------------------------------------------
// siginfo_t si_code 值（子进程状态原因）
// 与 Linux 内核 include/uapi/linux/siginfo.h 中 CLD_* 一致
// ---------------------------------------------------------------------------

/// 子进程已退出（正常终止）。
const CLD_EXITED: c_int = 1;

/// 子进程被信号杀死。
const CLD_KILLED: c_int = 2;

/// 子进程因信号终止并产生 core dump。
const CLD_DUMPED: c_int = 3;

/// 子进程被 traced（因 ptrace 停止）。
const CLD_TRAPPED: c_int = 4;

/// 子进程已停止（SIGSTOP 等）。
const CLD_STOPPED: c_int = 5;

/// 子进程已继续（SIGCONT 恢复执行）。
const CLD_CONTINUED: c_int = 6;

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 将 POSIX wait4 的 pid 参数映射为 waitid 的 idtype_t。
///
/// POSIX wait4 的 pid 语义：
/// - `pid < -1`: 等待进程组 ID 为 `-pid` 的任意子进程 → P_PGID
/// - `pid == -1`: 等待任意子进程 → P_ALL
/// - `pid == 0`: 等待与调用者同进程组的任意子进程 → P_PGID
/// - `pid > 0`: 等待指定进程 ID 的子进程 → P_PID
///
/// # 参数
///
/// * `pid` - POSIX wait4 的 pid 参数
///
/// # 返回值
///
/// 返回 `(idtype, id)` 元组：
/// * `idtype` — `P_PID` / `P_PGID` / `P_ALL`
/// * `id` — 修正后的进程/进程组标识符（对 P_PGID 和 P_PID 有效）
///
/// # 不变量
///
/// * `pid < -1`: idtype = P_PGID, id = -pid（取绝对值）
/// * `pid == -1`: idtype = P_ALL, id = 0
/// * `pid == 0`: idtype = P_PGID, id = 0（调用 getpgid(0) 由 SYS_waitid 支持）
/// * `pid > 0`: idtype = P_PID, id = pid
fn map_pid_to_idtype(pid: c_int) -> (c_int, c_int) {
    if pid < -1 {
        (P_PGID, -pid)
    } else if pid == -1 {
        (P_ALL, 0)
    } else if pid == 0 {
        (P_PGID, 0)
    } else {
        // pid > 0
        (P_PID, pid)
    }
}

/// 将 `siginfo_t` 的 `si_code` 和 `si_status` 编码为传统 UNIX wait 状态字。
///
/// wait 状态字的位布局（POSIX）：
/// - 低 7 位: 终止信号编号（若被信号杀死）或 0x7f（若被停止）
/// - 第 7 位 (0x80): core dump 标志
/// - 高位: 退出状态（`WEXITSTATUS`）
///
/// # 参数
///
/// * `si_code` — `siginfo_t.si_code`，子进程状态原因（CLD_EXITED 等）
/// * `si_status` — `siginfo_t.si_status`，子进程的退出码或信号编号
///
/// # 返回值
///
/// 传统 UNIX wait 状态字的编码值，可由 `WIFEXITED` / `WEXITSTATUS` /
/// `WIFSIGNALED` / `WTERMSIG` / `WCOREDUMP` / `WIFSTOPPED` /
/// `WSTOPSIG` / `WIFCONTINUED` 等宏解析。
///
/// # 编码规则
///
/// | si_code       | 编码公式                        | 语义                |
/// |---------------|---------------------------------|---------------------|
/// | CLD_EXITED    | `(status & 0xff) << 8`          | 正常退出            |
/// | CLD_KILLED    | `status & 0x7f`                 | 被信号杀死          |
/// | CLD_DUMPED    | `(status & 0x7f) \| 0x80`       | 被信号杀死 + core   |
/// | CLD_STOPPED   | `(status << 8) + 0x7f`          | 被停止              |
/// | CLD_TRAPPED   | `(status << 8) + 0x7f`          | 因 ptrace 停止      |
/// | CLD_CONTINUED | `0xffff`                        | 恢复执行            |
fn encode_wait_status(si_code: c_int, si_status: c_int) -> c_int {
    match si_code {
        CLD_EXITED => (si_status & 0xff) << 8,
        CLD_KILLED => si_status & 0x7f,
        CLD_DUMPED => (si_status & 0x7f) | 0x80,
        // CLD_STOPPED / CLD_TRAPPED: 保留高位的 PTRACE_EVENT_* 值
        CLD_STOPPED | CLD_TRAPPED => (si_status << 8) + 0x7f,
        CLD_CONTINUED => 0xffff,
        // 未识别的 si_code：返回 si_status 本身作为保守处理
        _ => si_status,
    }
}

// ---------------------------------------------------------------------------
// 公共接口
// ---------------------------------------------------------------------------

/// 使用 `SYS_waitid` 系统调用来模拟 `wait4()`。
///
/// 实现了 pid → idtype_t 映射 + siginfo_t → wait status 转换的
/// 两步模拟逻辑，使上层调用可通过 `WIFEXITED`、`WEXITSTATUS`
/// 等宏解析返回状态。
///
/// # 参数
///
/// * `pid` - 目标进程标识符，遵循 POSIX `wait4` 语义
/// * `status` - 输出参数，接收退出状态字；可为 null
/// * `options` - `WNOHANG`、`WUNTRACED` 等选项标志
/// * `kru` - `struct rusage*`，可为 null
/// * `cp` - 取消点标志：0 = 不可取消，1 = 可取消点
///
/// # 返回值
///
/// * 成功时返回子进程 pid
/// * 失败时返回负值 errno
/// * 无匹配子进程时返回 0
///
/// # 前置条件
///
/// * `pid` 符合 POSIX wait4 语义
/// * `status` 可以为 null
/// * `kru` 可以为 null
/// * `cp` 为 0 或 1
///
/// # 系统算法
///
/// 1. PID 映射：`pid < -1` → `P_PGID`，`-1` → `P_ALL`，
///    `0` → `P_PGID`，`> 0` → `P_PID`
/// 2. 调用 `waitid` 系统调用（始终附加 `WEXITED` 标志）
/// 3. 将 `si_code` 和 `si_status` 编码为传统 UNIX wait 状态字
///
/// # Rust 实现说明
///
/// 当前 PID 映射和状态转换逻辑已完整实现。实际的 `SYS_waitid`
/// 系统调用部分标记为 `todo!()` — 需要 `syscall_num` 模块添加
/// `SYS_waitid` 常量（x86_64: 247）并在 `crate::syscall` 中提供调用支持。
pub fn __emulate_wait4(
    pid: c_int,
    _status: *mut c_int,
    options: c_int,
    kru: *mut c_void,
    cp: c_int,
) -> c_long {
    // 步骤 1: PID → idtype_t 映射
    let (idtype, id) = map_pid_to_idtype(pid);

    // 步骤 2: 构造 waitid 的 options
    // 始终附加 WEXITED 标志以确保 wait4 语义完整性
    let _waitid_options = options | WEXITED;

    // 步骤 3: 调用 waitid 系统调用
    // SYS_waitid 的签名（x86_64）:
    //   long sys_waitid(int which, pid_t pid, siginfo_t *infop, int options, struct rusage *ru);
    //
    // 系统调用号: x86_64 = 247, aarch64 = 95（未在 syscall_num.rs 中定义）
    //
    // TODO: 在 syscall_num.rs 中添加 SYS_waitid 常量后，
    // 将以下 todo!() 替换为实际的系统调用：
    //
    //   let r = if cp != 0 {
    //       // 可取消点版本（支持 pthread_cancel）
    //       syscall_cp(SYS_waitid, idtype, id, &mut info, waitid_options, kru)
    //   } else {
    //       syscall_raw(SYS_waitid, idtype, id, &mut info, waitid_options, kru)
    //   };
    //
    //   if r < 0 { return r as c_long; }
    //
    //   if info.si_pid != 0 && !status.is_null() {
    //       *status = encode_wait_status(info.si_code, info.si_status);
    //   }
    //
    //   return info.si_pid as c_long;
    //
    // 当前返回占位值，标记需要 syscall 基础设施支持：
    let _ = (idtype, id, kru, cp);
    // 静默 unused 警告
    todo!(
        "__emulate_wait4: 需要 syscall_num 模块添加 SYS_waitid \
         常量 (x86_64=247, aarch64=95) 并集成到 crate::syscall"
    )
}

#[cfg(test)]
mod tests {
    use rusl_core::test;

    // 测试 PID → idtype 映射：pid < -1。
    test!("map_pid_to_idtype_neg_process_group" {
        let (idtype, id) = super::map_pid_to_idtype(-5);
        assert_eq!(idtype, super::P_PGID);
        assert_eq!(id, 5);
    });

    // 测试 PID → idtype 映射：pid == -1（任意子进程）。
    test!("map_pid_to_idtype_all" {
        let (idtype, id) = super::map_pid_to_idtype(-1);
        assert_eq!(idtype, super::P_ALL);
        assert_eq!(id, 0);
    });

    // 测试 PID → idtype 映射：pid == 0（同进程组）。
    test!("map_pid_to_idtype_zero" {
        let (idtype, id) = super::map_pid_to_idtype(0);
        assert_eq!(idtype, super::P_PGID);
        assert_eq!(id, 0);
    });

    // 测试 PID → idtype 映射：pid > 0（特定进程）。
    test!("map_pid_to_idtype_positive" {
        let (idtype, id) = super::map_pid_to_idtype(42);
        assert_eq!(idtype, super::P_PID);
        assert_eq!(id, 42);
    });

    // 测试 wait 状态编码：CLD_EXITED（正常退出）。
    test!("encode_wait_status_exited" {
        // 退出码为 0
        let sw = super::encode_wait_status(super::CLD_EXITED, 0);
        // WIFEXITED(sw) = true, WEXITSTATUS(sw) = 0
        assert_eq!(sw, 0);

        // 退出码为 42
        let sw = super::encode_wait_status(super::CLD_EXITED, 42);
        assert_eq!(sw, 42 << 8);
    });

    // 测试 wait 状态编码：CLD_KILLED（被信号杀死，无 core dump）。
    test!("encode_wait_status_killed" {
        // 被 SIGTERM(15) 杀死
        let sw = super::encode_wait_status(super::CLD_KILLED, 15);
        assert_eq!(sw, 15);
    });

    // 测试 wait 状态编码：CLD_DUMPED（被信号杀死 + core dump）。
    test!("encode_wait_status_dumped" {
        // 被 SIGQUIT(3) 杀死并产生 core dump
        let sw = super::encode_wait_status(super::CLD_DUMPED, 3);
        assert_eq!(sw, 3 | 0x80);
    });

    // 测试 wait 状态编码：CLD_STOPPED（被停止）。
    test!("encode_wait_status_stopped" {
        // 被 SIGSTOP(19) 停止
        let sw = super::encode_wait_status(super::CLD_STOPPED, 19);
        // 高位 = 19, 低 7 位 = 0x7f
        assert_eq!(sw, (19 << 8) | 0x7f);
    });

    // 测试 wait 状态编码：CLD_TRAPPED（因 ptrace 停止）。
    test!("encode_wait_status_trapped" {
        let sw = super::encode_wait_status(super::CLD_TRAPPED, 5);
        assert_eq!(sw & 0x7f, 0x7f);
    });

    // 测试 wait 状态编码：CLD_CONTINUED（恢复执行）。
    test!("encode_wait_status_continued" {
        let sw = super::encode_wait_status(super::CLD_CONTINUED, 0);
        assert_eq!(sw, 0xffff);
    });

    // 验证所有 POSIX 常量值与 Linux 内核一致。
    test!("wait_constants_correct" {
        assert_eq!(super::P_PID, 1);
        assert_eq!(super::P_PGID, 2);
        assert_eq!(super::P_ALL, 0);
        assert_eq!(super::WEXITED, 4);
        assert_eq!(super::WNOHANG, 1);
        assert_eq!(super::WUNTRACED, 2);
        assert_eq!(super::CLD_EXITED, 1);
        assert_eq!(super::CLD_KILLED, 2);
        assert_eq!(super::CLD_DUMPED, 3);
        assert_eq!(super::CLD_TRAPPED, 4);
        assert_eq!(super::CLD_STOPPED, 5);
        assert_eq!(super::CLD_CONTINUED, 6);
    });
}