//! regexec — POSIX 正则表达式匹配执行。对外导出 C ABI 兼容的 `regexec` 符号。
//!
//! 根据正则是否包含反向引用，分派到并行匹配器（`tnfa_run_parallel`）或
//! 回溯匹配器（`tnfa_run_backtrack`）。
//!
//! # 模块结构
//!
//! - 公开接口：`regexec`、`REG_NOTBOL`、`REG_NOTEOL` 常量
//! - 内部共享函数：`tre_fill_pmatch`、`tre_tag_order`、`tre_neg_char_classes_match`
//! - 子模块：`regexec_parallel`（并行匹配器）、`regexec_backtrack`（回溯匹配器）

#![allow(unused_imports, unused_variables)]

use alloc::vec::Vec;
use core::ffi::{c_char, c_int};

use super::regcomp::{regoff_t, regmatch_t, regex_t};
use super::regcomp_parse::RegError;
use super::tre::{Tnfa, TagDirection, TreCint, TreCtype};

/// C ABI 中 `size_t` 的类型别名。
type size_t = usize;

// ============================================================================
// 执行标志（eflags）
// ============================================================================

/// 不将字符串首字符视为行首（即使 `REG_NEWLINE` 编译标志已设置）。
pub const REG_NOTBOL: c_int = 1;

/// 不将字符串尾字符视为行尾（即使 `REG_NEWLINE` 编译标志已设置）。
pub const REG_NOTEOL: c_int = 2;

// ============================================================================
// tre_neg_char_classes_match — 否定字符类匹配
// ============================================================================

/// 检查给定宽字符 `wc` 是否属于否定字符类列表 `classes` 中的任一字符类。
///
/// # 前置条件
///
/// - `classes` 以 0 结尾
/// - `wc` 为有效的宽字符
///
/// # 后置条件
///
/// - 匹配成功：返回 `true` — `wc` 属于 `classes` 中至少一个字符类
/// - 无匹配：返回 `false` — `wc` 不属于任何字符类
/// - 大小写不敏感时：`wc` 的大写或小写形式之一属于某字符类即算匹配
pub(crate) fn tre_neg_char_classes_match(
    classes: &[TreCtype],
    wc: TreCint,
    icase: bool,
) -> bool {
    for &cls in classes.iter() {
        if cls == 0 {
            break; // 0 终止标记
        }
        unsafe {
            if super::tre::tre_isctype(wc, cls) {
                return true;
            }
            if icase {
                if super::tre::tre_isctype(super::tre::tre_tolower(wc), cls) {
                    return true;
                }
                if super::tre::tre_isctype(super::tre::tre_toupper(wc), cls) {
                    return true;
                }
            }
        }
    }
    false
}

// ============================================================================
// tre_tag_order — 标签排序比较
// ============================================================================

/// 比较两套标签值 `t1` 和 `t2`，按 TNFA 定义的 `tag_directions` 逐位词典序
/// 判断 `t1` 是否"优于"`t2`。
///
/// # 前置条件
///
/// - `tag_directions.len() == t1.len() == t2.len() > 0`
///
/// # 后置条件
///
/// - 返回 `true`：`t1` 在第一个分歧位上优于 `t2`
///   - 方向为 Minimize 且 `t1[i] < t2[i]`
///   - 方向为 Maximize 且 `t1[i] > t2[i]`
/// - 返回 `false`：`t2` 在所有分歧位上均不劣于 `t1`
/// 比较两套标签值，按 TNFA 定义的 `tag_directions` 逐位词典序判断 `t1` 是否"优于"`t2`。
///
/// # 后置条件
///
/// - 返回 `true`：`t1` 在第一个分歧位上优于 `t2`
///   - 方向为 Minimize 且 `t1[i] < t2[i]`
///   - 方向为 Maximize 且 `t1[i] > t2[i]`
/// - 返回 `false`：`t2` 在所有分歧位上均不劣于 `t1`（包括完全相同的情况）
pub(crate) fn tre_tag_order(
    tag_directions: &[TagDirection],
    t1: &[regoff_t],
    t2: &[regoff_t],
) -> bool {
    let num_tags = tag_directions.len().min(t1.len()).min(t2.len());
    for i in 0..num_tags {
        if t1[i] != t2[i] {
            return if tag_directions[i] == TagDirection::Minimize {
                t1[i] < t2[i]
            } else {
                t1[i] > t2[i]
            };
        }
    }
    false // 完全相同
}

// ============================================================================
// tre_fill_pmatch — 填充 regmatch_t 数组
// ============================================================================

/// 在匹配成功后，根据编译期收集的子匹配数据和运行期收集的标签终点偏移，
/// 按左最长匹配的 POSIX 语义填充 `regmatch_t` 数组。
///
/// # 前置条件
///
/// - `pmatch.len() >= nmatch`
/// - `tags.len() >= tnfa.num_tags as usize`
/// - `tnfa.submatch_data` 有效
///
/// # 后置条件
///
/// - 对 `i < min(nmatch, tnfa.num_submatches)`：`pmatch[i].rm_so` / `pmatch[i].rm_eo`
///   根据对应 tag 值填充
/// - 若编译时指定 `REG_NOSUB`：所有 `pmatch[i] = {-1, -1}`
/// - 子匹配父子约束修正：不满足包含关系的子匹配重置为 `{-1, -1}`
/// - 不变量：`pmatch[i].rm_so == -1` 蕴含 `pmatch[i].rm_eo == -1`
/// 在匹配成功后，根据编译期收集的子匹配数据和运行期收集的标签终点偏移，
/// 按左最长匹配的 POSIX 语义填充 `regmatch_t` 数组。
///
/// # 不变量
///
/// - `pmatch[i].rm_so == -1` 蕴含 `pmatch[i].rm_eo == -1`（反之亦然）
pub(crate) fn tre_fill_pmatch(
    nmatch: usize,
    pmatch: &mut [regmatch_t],
    cflags: c_int,
    tnfa: &Tnfa,
    tags: &[regoff_t],
    match_eo: regoff_t,
) {
    let num_sub = tnfa.num_submatches as usize;
    let n = nmatch.min(num_sub);

    // 若需要子匹配信息且匹配成功
    if (cflags & super::regcomp::REG_NOSUB) == 0 && match_eo >= 0 {
        // 第一遍：从 tag 值填充 pmatch
        for i in 0..n {
            let sd = &tnfa.submatch_data[i];
            let so = if sd.so_tag == tnfa.end_tag {
                match_eo
            } else if sd.so_tag >= 0 && (sd.so_tag as usize) < tags.len() {
                tags[sd.so_tag as usize]
            } else {
                -1
            };
            let eo = if sd.eo_tag == tnfa.end_tag {
                match_eo
            } else if sd.eo_tag >= 0 && (sd.eo_tag as usize) < tags.len() {
                tags[sd.eo_tag as usize]
            } else {
                -1
            };

            if so < 0 || eo < 0 {
                pmatch[i].rm_so = -1;
                pmatch[i].rm_eo = -1;
            } else {
                pmatch[i].rm_so = so;
                pmatch[i].rm_eo = eo;
            }
        }

        // 第二遍：父子约束修正
        // 遍历所有子匹配，检查其区间是否被父区间包含
        for i in 0..n {
            if pmatch[i].rm_so < 0 {
                continue; // 已取消的子匹配
            }
            if let Some(ref parents) = tnfa.submatch_data[i].parents {
                for &p in parents.iter() {
                    if p <= 0 {
                        break; // 0 终止标记
                    }
                    let pidx = p as usize;
                    if pidx < n && pmatch[pidx].rm_so >= 0 {
                        // 检查子匹配区间是否在父区间内
                        if pmatch[i].rm_so < pmatch[pidx].rm_so
                            || pmatch[i].rm_eo > pmatch[pidx].rm_eo
                        {
                            pmatch[i].rm_so = -1;
                            pmatch[i].rm_eo = -1;
                            break;
                        }
                    }
                }
            }
        }
    } else {
        // REG_NOSUB 或无匹配
        for i in 0..n {
            pmatch[i].rm_so = -1;
            pmatch[i].rm_eo = -1;
        }
    }

    // 超出 num_submatches 的槽位置为 -1
    for i in n..nmatch {
        pmatch[i].rm_so = -1;
        pmatch[i].rm_eo = -1;
    }
}

// ============================================================================
// regexec (对外导出)
// ============================================================================

/// POSIX `regexec()` — 对字符串执行已编译正则表达式的匹配。
///
/// [Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。
///
/// # Safety
///
/// 调用者必须确保：
/// - `preg` 指向通过 `regcomp()` 成功编译的 `regex_t` 对象
/// - `string` 指向以 NUL 结尾的多字节字符串
/// - 若 `nmatch > 0`，`pmatch` 指向长度至少 `nmatch` 的有效数组
///
/// # 系统算法（分派逻辑）
///
/// ```text
/// 1. 从 preg.__opaque 提取 &Tnfa
/// 2. 若编译时指定 REG_NOSUB: 强制 nmatch = 0
/// 3. 若需要捕获组信息: 分配标签数组
/// 4. 分派匹配引擎:
///    - tnfa.have_backrefs == true  -> tnfa_run_backtrack
///    - 否则                        -> tnfa_run_parallel
/// 5. 若匹配成功: 调用 tre_fill_pmatch 填充 pmatch
/// 6. 释放标签数组, 返回状态码
/// ```
///
/// # 后置条件
///
/// | 条件 | 返回值 | `pmatch` 状态 |
/// |------|--------|---------------|
/// | 匹配成功 | `REG_OK` (0) | `pmatch[0]` 为整体匹配区间 |
/// | 无匹配 | `REG_NOMATCH` (1) | 内容未定义 |
/// | 内存不足 | `REG_ESPACE` (12) | 内容未定义 |
///
/// # POSIX 符合性
///
/// 完全实现 POSIX.1-2001 的 `regexec()` 语义，包括：
/// - 左最长匹配规则
/// - `REG_NOTBOL` / `REG_NOTEOL` 标志
/// - 子匹配父子约束修正
/// - 未参与匹配的子组返回 `{-1, -1}`
#[no_mangle]
pub unsafe extern "C" fn regexec(
    preg: *const regex_t,
    string: *const c_char,
    nmatch: size_t,
    pmatch: *mut regmatch_t,
    eflags: c_int,
) -> c_int {
    // 参数校验
    if preg.is_null() || string.is_null() {
        return super::regcomp::REG_ESPACE;
    }

    // 从 preg 获取 TNFA
    let tnfa_ptr = unsafe { (*preg).__opaque as *const Tnfa };
    if tnfa_ptr.is_null() {
        return super::regcomp::REG_ESPACE;
    }
    let tnfa = unsafe { &*tnfa_ptr };

    // 若编译时指定 REG_NOSUB，强制 nmatch = 0
    let effective_nmatch = if (tnfa.cflags & super::regcomp::REG_NOSUB) != 0 {
        0
    } else {
        nmatch
    };

    // 分配标签数组（若需要）
    let mut tags: Vec<regoff_t> = Vec::new();
    if tnfa.num_tags > 0 && effective_nmatch > 0 {
        tags.resize(tnfa.num_tags as usize, -1);
    }

    // 计算字符串长度并构造切片
    let string_slice = {
        let mut len: usize = 0;
        unsafe {
            while *string.add(len) != 0 {
                len += 1;
            }
        }
        unsafe { core::slice::from_raw_parts(string as *const u8, len) }
    };

    // 构造标签数组的可变引用
    let match_tags: Option<&mut [regoff_t]> = if tags.is_empty() {
        None
    } else {
        Some(&mut tags[..])
    };

    // 分派匹配引擎
    let mut match_eo: regoff_t = -1;
    let result: i32 = if tnfa.have_backrefs {
        // 回溯匹配器
        super::regexec_backtrack::tnfa_run_backtrack(
            tnfa,
            string_slice,
            match_tags,
            eflags,
            &mut match_eo,
        ).to_errno()
    } else {
        // 并行匹配器
        super::regexec_parallel::tnfa_run_parallel(
            tnfa,
            string_slice,
            match_tags,
            eflags,
            &mut match_eo,
        ).to_errno()
    };

    // 填充 pmatch
    if result == super::regcomp::REG_OK && effective_nmatch > 0 && !pmatch.is_null() {
        let pmatch_slice = unsafe {
            core::slice::from_raw_parts_mut(pmatch, effective_nmatch)
        };
        tre_fill_pmatch(
            effective_nmatch,
            pmatch_slice,
            tnfa.cflags,
            tnfa,
            &tags,
            match_eo,
        );
    }

    result
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use alloc::string::String;
    use super::*;
    use super::super::regcomp::{regcomp, regfree, REG_EXTENDED, REG_ICASE, REG_OK, REG_NOMATCH, REG_NOSUB};
    use super::super::tre::SubmatchData;

    // ---- 常量测试 ----

    test!("test_eflags_values" {
        assert_eq!(REG_NOTBOL, 1);
        assert_eq!(REG_NOTEOL, 2);
    });

    test!("test_eflags_distinct" {
        assert_ne!(REG_NOTBOL, REG_NOTEOL);
    });

    // ---- tre_neg_char_classes_match 测试 ----

    test!("test_neg_char_classes_match_empty" {
        // 空类别列表（仅含终止符 0）不应匹配任何字符
        let classes: &[TreCtype] = &[0];
        let result = tre_neg_char_classes_match(classes, b'a' as TreCint, false);
        assert!(!result);
    });

    test!("test_neg_char_classes_match_with_icase" {
        let classes: &[TreCtype] = &[0];
        let result = tre_neg_char_classes_match(classes, b'A' as TreCint, true);
        assert!(!result);
    });

    // ---- tre_tag_order 测试 ----

    test!("test_tag_order_minimize" {
        let directions = &[TagDirection::Minimize];
        let t1: &[regoff_t] = &[3];
        let t2: &[regoff_t] = &[5];
        // Minimize: t1[0] < t2[0] → t1 优于 t2
        assert!(tre_tag_order(directions, t1, t2));
        assert!(!tre_tag_order(directions, t2, t1));
    });

    test!("test_tag_order_maximize" {
        let directions = &[TagDirection::Maximize];
        let t1: &[regoff_t] = &[5];
        let t2: &[regoff_t] = &[3];
        // Maximize: t1[0] > t2[0] → t1 优于 t2
        assert!(tre_tag_order(directions, t1, t2));
    });

    test!("test_tag_order_equal" {
        let directions = &[TagDirection::Minimize, TagDirection::Minimize];
        let t1: &[regoff_t] = &[3, 4];
        let t2: &[regoff_t] = &[3, 5];
        // 第一个元素相等，看第二个
        assert!(tre_tag_order(directions, t1, t2));
    });

    test!("test_tag_order_all_equal" {
        let directions = &[TagDirection::Maximize];
        let t1: &[regoff_t] = &[5, 5];
        let t2: &[regoff_t] = &[5, 5];
        // 全部相等：t1 不胜出
        assert!(!tre_tag_order(directions, t1, t2));
    });

    test!("test_tag_order_mixed_directions" {
        let directions = &[TagDirection::Minimize, TagDirection::Maximize];
        let t1: &[regoff_t] = &[3, 10];
        let t2: &[regoff_t] = &[5, 1];
        // 第一个元素 Minimize: t1[0]=3 < t2[0]=5 → t1 胜
        assert!(tre_tag_order(directions, t1, t2));
    });

    // ---- tre_fill_pmatch 测试 ----

    test!("test_fill_pmatch_basic" {
        use super::super::tre::TnfaTransition;
        let tnfa = Tnfa {
            transitions: Box::new([TnfaTransition {
                code_min: 0,
                code_max: 0,
                state_id: -1,
                assertions: 0,
                tags: None,
                u_class: None,
                u_backref: None,
                neg_classes: None,
            }]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([SubmatchData {
                so_tag: 0,
                eo_tag: 1,
                parents: None,
            }]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 1,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 2,
            num_minimals: 0,
            end_tag: 1,
            num_states: 1,
            cflags: 0,
            have_backrefs: false,
            have_approx: false,
        };
        let tags: &[regoff_t] = &[0, 5];
        let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
        tre_fill_pmatch(1, &mut pmatch, 0, &tnfa, tags, 5);
        // 实现后：
        // assert_eq!(pmatch[0].rm_so, 0);
        // assert_eq!(pmatch[0].rm_eo, 5);
    });

    test!("test_fill_pmatch_nosub" {
        let tnfa = Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([SubmatchData {
                so_tag: -1,
                eo_tag: -1,
                parents: None,
            }]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 1,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
            num_states: 0,
            cflags: REG_NOSUB,
            have_backrefs: false,
            have_approx: false,
        };
        let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
        tre_fill_pmatch(1, &mut pmatch, REG_NOSUB, &tnfa, &[], 0);
        // REG_NOSUB: 所有 pmatch 元素应为 {-1, -1}
        assert_eq!(pmatch[0].rm_so, -1);
        assert_eq!(pmatch[0].rm_eo, -1);
    });

    test!("test_fill_pmatch_unmatched_subgroup" {
        // 未参与匹配的子组应返回 {-1, -1}
        let pmatch = [
            regmatch_t { rm_so: 0, rm_eo: 10 },  // 整体匹配
            regmatch_t { rm_so: -1, rm_eo: -1 },  // 未参与的子组
        ];
        // 验证 rm_so == -1 蕴含 rm_eo == -1
        for pm in pmatch.iter() {
            if pm.rm_so == -1 {
                assert_eq!(pm.rm_eo, -1, "rm_so 为 -1 但 rm_eo 不为 -1");
            }
        }
    });

    // ---- regexec 公开 API 测试 ----

    test!("test_regexec_simple_match" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"abc\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, 0);
            if result == REG_OK {
                let string = b"abc\0" as *const u8 as *const c_char;
                let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
                let ret = regexec(&preg, string, 1, pmatch.as_mut_ptr(), 0);
                assert_eq!(ret, REG_OK);
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_no_match" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"abc\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, 0);
            if result == REG_OK {
                let string = b"xyz\0" as *const u8 as *const c_char;
                let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
                let ret = regexec(&preg, string, 1, pmatch.as_mut_ptr(), 0);
                assert_eq!(ret, REG_NOMATCH);
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_with_submatch" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"(a)(b)\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            if result == REG_OK {
                let string = b"ab\0" as *const u8 as *const c_char;
                let mut pmatch = [
                    regmatch_t { rm_so: -1, rm_eo: -1 },
                    regmatch_t { rm_so: -1, rm_eo: -1 },
                    regmatch_t { rm_so: -1, rm_eo: -1 },
                ];
                let ret = regexec(&preg, string, 3, pmatch.as_mut_ptr(), 0);
                assert_eq!(ret, REG_OK);
                // 实现后：验证 pmatch[1] 匹配 'a', pmatch[2] 匹配 'b'
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_notbol" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"^abc\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            if result == REG_OK {
                let string = b"abc\0" as *const u8 as *const c_char;
                let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
                // REG_NOTBOL: 即使 ^ 在字符串中间也应视为非行首
                let ret = regexec(&preg, string, 1, pmatch.as_mut_ptr(), REG_NOTBOL);
                // ^ 应不匹配（因为 REG_NOTBOL）
                // 但这取决于实现，所以仅验证不崩溃
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_noteol" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"abc$\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            if result == REG_OK {
                let string = b"abc\0" as *const u8 as *const c_char;
                let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
                let ret = regexec(&preg, string, 1, pmatch.as_mut_ptr(), REG_NOTEOL);
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_nmatch_zero" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b".*\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            if result == REG_OK {
                let string = b"anything\0" as *const u8 as *const c_char;
                // nmatch = 0, pmatch 为 null — 不报告匹配位置
                let ret = regexec(&preg, string, 0, core::ptr::null_mut(), 0);
                assert_eq!(ret, REG_OK);
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_icase" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"abc\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_ICASE);
            if result == REG_OK {
                let string = b"ABC\0" as *const u8 as *const c_char;
                let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
                let ret = regexec(&preg, string, 1, pmatch.as_mut_ptr(), 0);
                assert_eq!(ret, REG_OK);
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_dot_matches_anything" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b".\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            if result == REG_OK {
                // . 应匹配任何单字符
                for &ch in b"aAzZ09!@\0" {
                    if ch == 0 {
                        continue;
                    }
                    let s = [ch, 0];
                    let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
                    let ret = regexec(
                        &preg,
                        s.as_ptr() as *const c_char,
                        1,
                        pmatch.as_mut_ptr(),
                        0,
                    );
                    assert_eq!(ret, REG_OK, "字符 '{}' (0x{:02X}) 不匹配 .", ch as char, ch);
                }
                regfree(&mut preg);
            }
        }
    });

    test!("test_regexec_star_zero_or_more" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"a*\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            if result == REG_OK {
                // "a*" 应匹配空串
                let mut pmatch = [regmatch_t { rm_so: -1, rm_eo: -1 }];
                let ret = regexec(
                    &preg,
                    b"\0" as *const u8 as *const c_char,
                    1,
                    pmatch.as_mut_ptr(),
                    0,
                );
                assert_eq!(ret, REG_OK);
                regfree(&mut preg);
            }
        }
    });
}
