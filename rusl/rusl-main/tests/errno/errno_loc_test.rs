/// 模块: errno_loc_test
/// `__errno_location` / `___errno_location` 集成测试
///
/// 基于 `spec/errno/__errno_location.md` 规约生成。
///
/// ## 测试覆盖
///
/// - __errno_location 返回非空指针
/// - 读写一致性
/// - 初始 errno 值
/// - 两别名返回相同地址
/// - 写入持久性
/// - 调用不修改 errno
/// - 各种 errno 值 (0, 正数, 负数, 边界值)

use core::ffi::c_int;

use rusl::api::errno::{__errno_location, ___errno_location};
use rusl_core::test;

// ===========================================================================
// 基本功能测试
// ===========================================================================

test!("test___errno_location_non_null" {
    // __errno_location 必须始终返回有效的非空指针
    let ptr = __errno_location();
    assert!(!ptr.is_null(), "__errno_location returned null pointer");
});

test!("test____errno_location_non_null" {
    // ___errno_location 也必须返回非空指针
    let ptr = ___errno_location();
    assert!(!ptr.is_null(), "___errno_location returned null pointer");
});

test!("test_aliases_same_address" {
    // 两个别名必须返回相同的地址 (指向同一个 errno 存储)
    let p1 = __errno_location();
    let p2 = ___errno_location();
    assert_eq!(p1, p2, "__errno_location and ___errno_location must return same address");
});

// ===========================================================================
// errno 初始值和读写测试
// ===========================================================================

test!("test_errno_initial_value" {
    // 初始 errno 应为 0 (musl 在程序启动时初始化 errno 为 0)
    let ptr = __errno_location();
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 0, "initial errno should be 0, got {}", val);
});

test!("test_errno_write_read_cycle" {
    // 写入后读取应一致
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 42) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 42, "errno write/read mismatch: wrote 42, read {}", val);
});

test!("test_errno_write_via_main_read_via_alias" {
    // 通过 __errno_location 写入, 通过 ___errno_location 读取
    let p_main = __errno_location();
    let p_alias = ___errno_location();
    unsafe { core::ptr::write(p_main, 77) };
    let val = unsafe { core::ptr::read(p_alias) };
    assert_eq!(val, 77, "write via __errno_location, read via ___errno_location: got {}", val);
});

test!("test_errno_write_via_alias_read_via_main" {
    // 通过 ___errno_location 写入, 通过 __errno_location 读取
    let p_main = __errno_location();
    let p_alias = ___errno_location();
    unsafe { core::ptr::write(p_alias, 88) };
    let val = unsafe { core::ptr::read(p_main) };
    assert_eq!(val, 88, "write via ___errno_location, read via __errno_location: got {}", val);
});

test!("test_errno_reset_to_zero" {
    // 写入非零值后重置为 0
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 100) };
    unsafe { core::ptr::write(ptr, 0) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 0, "errno reset to 0 failed, got {}", val);
});

// ===========================================================================
// 持久性和不变性测试
// ===========================================================================

test!("test_errno_persists_between_reads" {
    // errno 值在对 __errno_location 的多次调用之间保持不变
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 55) };
    // 再次调用 __errno_location 不应改变 errno
    let ptr2 = __errno_location();
    assert_eq!(ptr, ptr2, "pointer should be stable across calls");
    let val = unsafe { core::ptr::read(ptr2) };
    assert_eq!(val, 55, "errno changed unexpectedly, got {}", val);
});

test!("test_calling___errno_location_preserves_errno" {
    // 调用 ___errno_location 不应修改 errno
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 33) };
    // 调用 ___errno_location
    let _ = ___errno_location();
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 33, "___errno_location modified errno, got {}", val);
});

test!("test_calling___errno_location_preserves_errno_multiple" {
    // 多次调用 ___errno_location 不应改变 errno
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 99) };
    // 多次调用
    for _ in 0..10 {
        let _ = ___errno_location();
    }
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 99, "repeated ___errno_location calls modified errno, got {}", val);
});

// ===========================================================================
// 各种 errno 值测试
// ===========================================================================

test!("test_errno_einval" {
    // EINVAL = 22
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 22) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 22, "EINVAL value mismatch, got {}", val);
});

test!("test_errno_enomem" {
    // ENOMEM = 12
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 12) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 12, "ENOMEM value mismatch, got {}", val);
});

test!("test_errno_eacces" {
    // EACCES = 13
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 13) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 13, "EACCES value mismatch, got {}", val);
});

test!("test_errno_enoent" {
    // ENOENT = 2
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 2) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 2, "ENOENT value mismatch, got {}", val);
});

test!("test_errno_negative_value" {
    // errno 可以存储负值 (虽然不常见)
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, -1) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, -1, "negative errno value mismatch, got {}", val);
});

test!("test_errno_large_value" {
    // errno 可以存储较大的值
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 0x7FFFFFFF) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 0x7FFFFFFF, "large errno value mismatch, got {}", val);
});

test!("test_errno_zero" {
    // errno = 0 (成功)
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 0) };
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 0, "errno zero value mismatch, got {}", val);
});

test!("test_errno_same_address_after_write" {
    // 写入后地址不变
    let ptr1 = __errno_location();
    unsafe { core::ptr::write(ptr1, 12345) };
    let ptr2 = __errno_location();
    assert_eq!(ptr1, ptr2, "address should not change after write");
});

test!("test_errno_sequential_writes" {
    // 连续写入不同值
    let ptr = __errno_location();
    let values: &[c_int] = &[0, 1, 2, 5, 10, 22, 100, 0];
    for &v in values {
        unsafe { core::ptr::write(ptr, v) };
        let val = unsafe { core::ptr::read(ptr) };
        assert_eq!(val, v, "sequential write/read: wrote {}, read {}", v, val);
    }
});

test!("test_errno_alias_sequential_writes" {
    // 通过两个别名交替写入
    let p1 = __errno_location();
    let p2 = ___errno_location();
    unsafe { core::ptr::write(p1, 10) };
    assert_eq!(unsafe { core::ptr::read(p2) }, 10);
    unsafe { core::ptr::write(p2, 20) };
    assert_eq!(unsafe { core::ptr::read(p1) }, 20);
    unsafe { core::ptr::write(p1, 30) };
    assert_eq!(unsafe { core::ptr::read(p2) }, 30);
});

// ===========================================================================
// 函数调用不修改 errno 测试
// ===========================================================================

test!("test___errno_location_does_not_modify_errno" {
    // __errno_location 本身不应修改 errno
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 42) };
    // 再次调用
    let _ = __errno_location();
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 42, "__errno_location modified errno, got {}", val);
});

test!("test____errno_location_does_not_modify_errno" {
    // ___errno_location 本身不应修改 errno
    let ptr = __errno_location();
    unsafe { core::ptr::write(ptr, 17) };
    // 调用 ___errno_location
    let _ = ___errno_location();
    let val = unsafe { core::ptr::read(ptr) };
    assert_eq!(val, 17, "___errno_location modified errno, got {}", val);
});