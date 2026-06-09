//! __ctype_get_mb_cur_max — 返回当前 locale 下多字节字符的最大字节数。
//! 对应 musl src/ctype/__ctype_get_mb_cur_max.c
//!
//! 该函数是 `<stdlib.h>` 中 `MB_CUR_MAX` 宏的函数级实现。
//! `MB_CUR_MAX` 宏展开为 `__ctype_get_mb_cur_max()` 调用，
//! 用于查询当前 locale 下单个多字节字符的最大字节数。

#![allow(unused_imports, unused_variables)]

/// 返回当前 locale 下多字节字符的最大字节数（即 `MB_CUR_MAX` 宏的值）。
///
/// # 返回值
///
/// | Locale 类型 | 返回值 | 说明 |
/// |-----------|--------|------|
/// | UTF-8     | 4      | UTF-8 编码最多 4 字节 |
/// | C/POSIX   | 1      | 单字节编码 |
/// | 其他单字节 | 1      | 其他单字节编码 |
///
/// # Safety
///
/// 此函数无前置条件，可在任何时刻调用。标记为 `unsafe` 是为了与 C ABI 兼容。
///
/// # 不变量
///
/// 返回值仅依赖于当前线程的 locale 设置。在 locale 未变更期间，多次调用返回相同值。
///
/// # 系统算法
///
/// ```text
/// 读取当前线程 locale 结构中的 LC_CTYPE 类别指针。
/// 若设置了 cat[LC_CTYPE]（即 locale 非 C locale），返回 4（UTF-8）；
/// 否则返回 1（单字节编码）。
/// 时间复杂度 O(1)。
/// ```
///
/// # 复杂度
///
/// 时间复杂度 O(1)。
#[no_mangle]
pub extern "C" fn __ctype_get_mb_cur_max() -> usize {
    // 在 C locale (默认) 下，MB_CUR_MAX = 1。
    // musl 中 MB_CUR_MAX 宏展开为 (CURRENT_UTF8 ? 4 : 1)，
    // 其中 CURRENT_UTF8 检查当前线程 locale 的 LC_CTYPE 类别。
    //
    // 当前 rusl no_std 环境尚未实现完整的 locale/线程支持，
    // 因此始终返回 C locale 默认值 1。
    // 当后续添加 UTF-8 locale 支持时，需要读取线程本地 locale 结构。
    1
}
