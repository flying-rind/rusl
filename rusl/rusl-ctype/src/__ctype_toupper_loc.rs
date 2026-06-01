//! __ctype_toupper_loc — 返回 C locale 下字符大写映射表的地址。
//! 对应 musl src/ctype/__ctype_toupper_loc.c
//!
//! 该函数被 `<ctype.h>` 中的 `toupper()` 宏/函数用于 O(1) 字符大小写转换。
//! 返回指向内部静态映射表的指针，表中每个元素为 `i32` 类型，
//! 存储目标大写字符的码位值。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;

// ============================================================================
// 静态大写映射表 (编译期构造)
// ============================================================================

/// 编译期构建 384 元素的大写映射表。
///
/// 表组织方式：
/// - `table[0..127]`: 索引 -128 到 -1（128 个条目，全为零）
/// - `table[128..255]`: 索引 0 到 127（ASCII 范围）
///   - 小写字母 'a'-'z' (97-122) → 对应大写字母 'A'-'Z' (65-90)
///   - 其余字符恒等映射
/// - `table[256..383]`: 索引 128 到 255（扩展 ASCII，全为零）
const fn build_toupper_table() -> [c_int; 384] {
    let mut table: [c_int; 384] = [0i32; 384];
    // 只填充 ASCII 范围区间: table[128..256] 对应字符索引 0..127
    let mut i: usize = 128;
    while i < 256 {
        let idx: usize = i - 128; // 字符码位 (0..127)
        table[i] = if idx >= 97 && idx <= 122 {
            // 小写字母 'a'-'z': 映射到对应大写字母
            (idx - 32) as c_int
        } else {
            // 大写字母或其他字符: 恒等映射
            idx as c_int
        };
        i += 1;
    }
    table
}

/// C/POSIX locale 下的静态大写映射表。
///
/// 共 384 个 `i32` 元素，集中存放于只读数据段。
/// 通过 `ptable = &table[128]` 提供 +128 偏移的索引语义，
/// 使调用者可用 `ptable[-128..255]` 访问有效数据。
static TOUPPER_TABLE: [c_int; 384] = build_toupper_table();

// ============================================================================
// 公开导出接口 (C ABI 兼容)
// ============================================================================

/// 返回 C locale 下字符大写映射表的指针的指针。
///
/// 解引用一次得到表指针 `table_ptr`，该表有 384 个 `i32` 元素，
/// 索引偏移量为 +128（即 `table_ptr[-128]` 到 `table_ptr[255]` 有效）。
/// 实际使用时，调用者以字符码位为索引直接查询：`(*table_ptr)[c]` 得到字符 `c` 的大写映射。
///
/// # 返回值
///
/// 返回 `*const *const i32`，指向内部静态大写映射表指针。
/// 返回的指针在整个程序生命周期内有效，指向只读数据段。
///
/// # 映射语义
///
/// 对任意有效索引 `idx`（-128 到 255），`table_ptr[idx]` 的值语义为：
/// - 若 `idx` 对应小写字母（`'a'`-`'z'`，即 97-122），返回对应大写字母的码位值（`'A'`-`'Z'`，即 65-90）。
/// - 若 `idx` 对应大写字母或其他字符，返回 `idx` 本身（恒等映射）。
///
/// # Safety
///
/// 此函数始终返回有效指针。标记为 `unsafe` 是因为：
/// - 返回原始指针，调用者需自行保证索引在 [-128, 255] 范围内。
/// - 表内容为只读，不可通过返回的指针修改。
///
/// # 不变量
///
/// 纯函数，始终返回同一常量指针。无内部可变状态。映射表内容永不改变。
/// 仅实现 C locale 的大小写映射，不依赖运行时 locale。
///
/// # 复杂度
///
/// 时间复杂度 O(1)。
#[no_mangle]
pub unsafe extern "C" fn __ctype_toupper_loc() -> *const *const i32 {
    // ToupperTablePtr 手动实现 Sync, 避免 `*const i32` 不满足 static Sync 约束的问题
    // 使用 repr(transparent) 确保内存布局与 *const i32 完全一致, C ABI 兼容
    #[repr(transparent)]
    struct ToupperTablePtr(*const i32);
    // Safety: 该指针指向只读静态数据段，可安全地在多线程间共享
    unsafe impl Sync for ToupperTablePtr {}

    static PTR: ToupperTablePtr = ToupperTablePtr(unsafe { TOUPPER_TABLE.as_ptr().add(128) });
    &raw const PTR as *const *const i32
}
