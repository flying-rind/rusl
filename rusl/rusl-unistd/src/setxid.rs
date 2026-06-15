//! setxid — 跨所有线程原子性地执行 UID/GID 设置系统调用。
//! 对应 musl src/unistd/setxid.c
//!
//! 这是 musl 实现 `setuid`/`setgid` 族的核心基础设施。
//! 简化实现：直接执行系统调用（单线程场景足够，多线程安全需额外 synccall 机制）。

use core::ffi::c_int;
use rusl_internal::syscall::raw_syscall3;
use rusl_internal::syscall::__syscall_ret;

/// `__setxid` — musl 内部 `hidden` 函数。
///
/// 执行 UID/GID 设置系统调用。所有 `setuid`/`seteuid`/`setgid`/`setegid`/
/// `setreuid`/`setregid`/`setresuid`/`setresgid` 均通过此函数实现。
///
/// 参数:
/// - `nr`: 系统调用编号
/// - `id`: 真实 ID，-1 表示不修改
/// - `eid`: 有效 ID，-1 表示不修改
/// - `sid`: 保存的 set-ID，-1 表示不修改
#[no_mangle]
/// [Visibility]: Internal
pub extern "C" fn __setxid(nr: c_int, id: c_int, eid: c_int, sid: c_int) -> c_int {
    // 简化实现：直接执行系统调用
    // 完整实现需要 __synccall 跨线程同步
    unsafe {
        let r = raw_syscall3(nr as i64, id as i64, eid as i64, sid as i64) as u64;
        __syscall_ret(r) as c_int
    }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_setxid_noop_returns_zero" {
        // 使用 setresuid 将所有 ID 设置为 -1（不修改），应该成功返回 0
        // 这是无操作调用，不需要特权
        let ret = __setxid(
            rusl_internal::syscall::SYS_setresuid as c_int,
            -1, -1, -1,
        );
        assert_eq!(ret, 0, "__setxid(SYS_setresuid, -1, -1, -1) should succeed (no-op)");
    });

    test!("test_setxid_setresgid_noop" {
        // 同样对 setresgid 做无操作调用
        let ret = __setxid(
            rusl_internal::syscall::SYS_setresgid as c_int,
            -1, -1, -1,
        );
        assert_eq!(ret, 0, "__setxid(SYS_setresgid, -1, -1, -1) should succeed (no-op)");
    });

    test!("test_setxid_invalid_syscall" {
        // 使用无效的系统调用号：应返回 -1
        let ret = __setxid(99999, -1, -1, -1);
        assert_eq!(ret, -1, "__setxid with invalid syscall number should return -1");
    });

    test!("test_setxid_setreuid_noop" {
        // setreuid(-1, -1): 不修改真实和有效 UID
        let ret = __setxid(
            rusl_internal::syscall::SYS_setreuid as c_int,
            -1, -1, -1,
        );
        assert_eq!(ret, 0, "__setxid(SYS_setreuid, -1, -1, -1) should succeed (no-op)");
    });

    test!("test_setxid_setregid_noop" {
        // setregid(-1, -1): 不修改真实和有效 GID
        let ret = __setxid(
            rusl_internal::syscall::SYS_setregid as c_int,
            -1, -1, -1,
        );
        assert_eq!(ret, 0, "__setxid(SYS_setregid, -1, -1, -1) should succeed (no-op)");
    });

    test!("test_setxid_consistency_same_syscall" {
        // 多次无操作调用应返回相同的成功结果
        let r1 = __setxid(
            rusl_internal::syscall::SYS_setresuid as c_int,
            -1, -1, -1,
        );
        let r2 = __setxid(
            rusl_internal::syscall::SYS_setresuid as c_int,
            -1, -1, -1,
        );
        assert_eq!(r1, r2, "consecutive no-op calls should return same result");
        assert_eq!(r1, 0);
    });
}
