//! regexec 并行 NFA 模拟匹配器 — `tnfa_run_parallel`。
//!
//! 实现 POSIX 左最长匹配的并行 NFA 模拟算法。所有匹配路径同时推进，
//! 到达同一状态时按标签方向规则择一保留。
//!
//! **该算法不能处理包含反向引用的正则表达式**（此时应使用 `tnfa_run_backtrack`）。
//!
//! 所有符号均为 `pub(crate)` 可见性。

#![allow(unused_imports, unused_variables)]

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_int;

use super::regcomp::regoff_t;
use super::regcomp_parse::RegError;
use super::tre::{Tnfa, TnfaTransition, TagDirection, TreCint, TreCtype, END_STATE_MARKER};
use super::regexec::{tre_neg_char_classes_match, tre_tag_order, REG_NOTBOL, REG_NOTEOL};
use super::regcomp::{REG_ICASE, REG_NEWLINE};

// ============================================================================
// ReachState — 并行匹配器的可达状态
// ============================================================================

/// 并行匹配器（`tnfa_run_parallel`）中某一可达路径的状态。
#[derive(Clone, Debug)]
pub(crate) struct ReachState {
    /// 当前所处 TNFA 状态的 ID。
    pub state_id: i32,
    /// 标签值数组（记录各捕获组的起始偏移），长度等于 `tnfa.num_tags`。
    pub tags: Box<[regoff_t]>,
}

// ============================================================================
// ReachPos — 每状态已访问记录
// ============================================================================

/// 记录 TNFA 某个 `state_id` 最近一次被访问时的字符位置和最佳标签数组。
#[derive(Clone, Debug)]
pub(crate) struct ReachPos {
    /// 该状态最近一次被访问时的字符位置。
    pub pos: regoff_t,
    /// 该状态对应的最佳标签值数组。None 表示尚未访问过。
    pub tags: Option<Box<[regoff_t]>>,
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 获取 TNFA 中指定状态的所有出边转移。
///
/// 扫描 transitions 数组，收集所有从 `state_id` 出发的转移。
/// 状态边界由 `state_id == END_STATE_MARKER` 标记。
fn get_state_transitions<'a>(
    transitions: &'a [TnfaTransition],
    state_id: i32,
) -> &'a [TnfaTransition] {
    // 扫描找到该状态的转移起始位置
    let mut current_state: i32 = 0;
    let mut start_idx: usize = 0;

    while current_state < state_id && start_idx < transitions.len() {
        // 找到当前状态的终止标记
        while start_idx < transitions.len() && transitions[start_idx].state_id != END_STATE_MARKER {
            start_idx += 1;
        }
        // 跳过终止标记
        if start_idx < transitions.len() {
            start_idx += 1;
        }
        current_state += 1;
    }

    if start_idx >= transitions.len() {
        return &[];
    }

    // 收集该状态的所有出边
    let end_idx = start_idx;
    let mut count = 0usize;
    while start_idx + count < transitions.len()
        && transitions[start_idx + count].state_id != END_STATE_MARKER
    {
        count += 1;
    }

    &transitions[start_idx..start_idx + count]
}

/// 获取 TNFA 的初始转移（即状态 0 的所有出边）。
fn get_initial_transitions(tnfa: &Tnfa) -> &[TnfaTransition] {
    get_state_transitions(&tnfa.transitions, tnfa.initial_id)
}

/// 检查断言条件。
///
/// pos: 当前字符位置（-1 表示尚未读取字符）
/// prev_c: 前一个宽字符（用于词边界断言）
/// next_c: 当前宽字符
/// eflags: 执行标志
/// tnfa: 已编译的 TNFA
fn check_assertions(
    assertions: i32,
    pos: regoff_t,
    prev_c: TreCint,
    next_c: TreCint,
    eflags: c_int,
    tnfa: &Tnfa,
) -> bool {
    let reg_notbol = (eflags & REG_NOTBOL) != 0;
    let reg_noteol = (eflags & REG_NOTEOL) != 0;
    let reg_newline = (tnfa.cflags & REG_NEWLINE) != 0;

    if (assertions & super::tre::ASSERT_AT_BOL) != 0 {
        if pos > 0 || reg_notbol {
            if !reg_newline || prev_c != b'\n' as TreCint {
                return true; // 断言失败
            }
        }
    }
    if (assertions & super::tre::ASSERT_AT_EOL) != 0 {
        if next_c != 0 || reg_noteol {
            if !reg_newline || next_c != b'\n' as TreCint {
                return true; // 断言失败
            }
        }
    }
    if (assertions & super::tre::ASSERT_AT_BOW) != 0 {
        let prev_word = prev_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(prev_c) };
        let next_word = next_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(next_c) };
        if prev_word || !next_word {
            return true; // 断言失败
        }
    }
    if (assertions & super::tre::ASSERT_AT_EOW) != 0 {
        let prev_word = prev_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(prev_c) };
        let next_word = next_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(next_c) };
        if !prev_word || next_word {
            return true; // 断言失败
        }
    }
    if (assertions & super::tre::ASSERT_AT_WB) != 0 {
        if pos != 0 && next_c != 0 {
            let prev_word = prev_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(prev_c) };
            let next_word = next_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(next_c) };
            if prev_word == next_word {
                return true; // 断言失败
            }
        }
    }
    if (assertions & super::tre::ASSERT_AT_WB_NEG) != 0 {
        if pos == 0 || next_c == 0 {
            return true; // 断言失败
        }
        let prev_word = prev_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(prev_c) };
        let next_word = next_c == '_' as TreCint || unsafe { super::tre::tre_isalnum(next_c) };
        if prev_word != next_word {
            return true; // 断言失败
        }
    }
    false // 断言通过
}

/// 检查字符类。
fn check_char_classes(trans: &TnfaTransition, prev_c: TreCint, tnfa: &Tnfa) -> bool {
    if (trans.assertions & super::tre::ASSERT_CHAR_CLASS) != 0 {
        let icase = (tnfa.cflags & REG_ICASE) != 0;
        if !icase {
            if let Some(cls) = trans.u_class {
                if unsafe { !super::tre::tre_isctype(prev_c, cls) } {
                    return true; // 不匹配
                }
            }
        } else {
            if let Some(cls) = trans.u_class {
                let lc = unsafe { super::tre::tre_tolower(prev_c) };
                let uc = unsafe { super::tre::tre_toupper(prev_c) };
                if unsafe { !super::tre::tre_isctype(lc, cls) && !super::tre::tre_isctype(uc, cls) } {
                    return true; // 不匹配
                }
            }
        }
    }
    if (trans.assertions & super::tre::ASSERT_CHAR_CLASS_NEG) != 0 {
        if let Some(ref neg_classes) = trans.neg_classes {
            if tre_neg_char_classes_match(neg_classes, prev_c, (tnfa.cflags & REG_ICASE) != 0) {
                return true; // 匹配否定类
            }
        }
    }
    false // 不排除此转移
}

// ============================================================================
// tnfa_run_parallel — 并行 NFA 模拟匹配器
// ============================================================================

/// 实现 POSIX 左最长匹配的并行 NFA 模拟算法。
///
/// 所有匹配路径同时推进，到达同一状态时按标签方向规则择一保留。
pub(crate) fn tnfa_run_parallel(
    tnfa: &Tnfa,
    string: &[u8],
    mut match_tags: Option<&mut [regoff_t]>,
    eflags: c_int,
    match_end_ofs: &mut regoff_t,
) -> RegError {
    let num_tags = if match_tags.is_some() { tnfa.num_tags as usize } else { 0 };
    let num_states = tnfa.num_states.max(1) as usize;

    // 分配可达状态数组
    let max_reach = num_states + 1;
    let mut reach: Vec<ReachState> = Vec::with_capacity(max_reach);
    let mut reach_next: Vec<ReachState> = Vec::with_capacity(max_reach);

    // 初始化 reach_pos
    let mut reach_pos: Vec<ReachPos> = Vec::with_capacity(num_states);
    for _ in 0..num_states {
        reach_pos.push(ReachPos { pos: -1, tags: None });
    }

    // 临时标签数组
    let mut tmp_tags: Vec<regoff_t> = Vec::with_capacity(num_tags);
    if num_tags > 0 {
        tmp_tags.resize(num_tags, -1);
    }

    // 状态变量
    let mut prev_c: TreCint = 0;
    let mut next_c: TreCint = 0;
    let mut str_pos: usize = 0;
    let mut pos: regoff_t;
    let mut pos_add_next: regoff_t = 1;
    let mut match_eo: regoff_t = -1;
    let mut new_match = false;

    // 读取第一个宽字符
    if !string.is_empty() {
        let mut wc: i32 = 0;
        let len = unsafe {
            super::tre::tre_mbtowc(&mut wc, string.as_ptr(), string.len().min(super::tre::MB_LEN_MAX))
        };
        if len < 0 {
            *match_end_ofs = -1;
            return RegError::NoMatch;
        }
        next_c = if len > 0 { wc } else { 0 };
        pos_add_next = if len > 0 { len as regoff_t } else { 1 };
        str_pos = if len > 0 { len as usize } else { 0 };
    }
    pos = 0;

    // 主循环
    loop {
        // 若尚未找到匹配，添加初始状态
        if match_eo < 0 {
            let initial_trans = get_initial_transitions(tnfa);
            for trans in initial_trans {
                if trans.assertions != 0
                    && check_assertions(trans.assertions, pos, prev_c, next_c, eflags, tnfa)
                {
                    continue;
                }
                let dest = trans.state_id;
                if dest >= 0 && (dest as usize) < num_states && reach_pos[dest as usize].pos < pos {
                    let mut tags: Box<[regoff_t]> = vec![-1i64; num_tags].into_boxed_slice();
                    if let Some(ref trans_tags) = trans.tags {
                        for &t in trans_tags.iter() {
                            if t >= 0 && (t as usize) < num_tags {
                                tags[t as usize] = pos;
                            }
                        }
                    }
                    if dest == tnfa.final_id {
                        match_eo = pos;
                        new_match = true;
                        if let Some(ref mut mt) = match_tags {
                            let n = mt.len().min(tags.len());
                            mt[..n].copy_from_slice(&tags[..n]);
                        }
                    }
                    reach_pos[dest as usize].pos = pos;
                    reach_pos[dest as usize].tags = Some(tags.clone());
                    reach_next.push(ReachState { state_id: dest, tags });
                }
            }
        } else {
            // 已找到匹配，检查是否应终止
            if num_tags == 0 || reach_next.is_empty() {
                break;
            }
        }

        // 检查字符串末尾
        if next_c == 0 && str_pos == 0 && match_eo < 0 {
            break;
        }
        if next_c == 0 && str_pos >= string.len() {
            break;
        }

        // 读取下一宽字符
        prev_c = next_c;
        pos += pos_add_next;
        if str_pos >= string.len() {
            next_c = 0;
            pos_add_next = 1;
        } else {
            let mut wc: i32 = 0;
            let remaining = &string[str_pos..];
            let len = unsafe {
                super::tre::tre_mbtowc(&mut wc, remaining.as_ptr(),
                    remaining.len().min(super::tre::MB_LEN_MAX))
            };
            if len <= 0 {
                next_c = 0;
                pos_add_next = 1;
                if len < 0 {
                    break;
                }
            } else {
                next_c = wc;
                pos_add_next = len as regoff_t;
                str_pos += len as usize;
            }
        }

        // 交换 reach 和 reach_next
        core::mem::swap(&mut reach, &mut reach_next);
        reach_next.clear();

        // 最小匹配剔除
        if tnfa.num_minimals > 0 && new_match {
            new_match = false;
            reach.retain(|rs| {
                if let Some(ref mt) = tnfa.minimal_tags {
                    let mut i = 0;
                    while i + 1 < mt.len() && mt[i] >= 0 {
                        let end = mt[i] as usize;
                        let start = mt[i + 1] as usize;
                        if end < num_tags && start < num_tags {
                            if let Some(ref mtags) = match_tags {
                                if rs.tags[start] == mtags[start]
                                    && rs.tags[end] < mtags[end]
                                {
                                    return false;
                                }
                            }
                        }
                        i += 2;
                    }
                }
                true
            });
            core::mem::swap(&mut reach, &mut reach_next);
            reach_next.clear();
        }

        // 探索每个可达状态的出边
        for rs in reach.iter() {
            let state_trans = get_state_transitions(&tnfa.transitions, rs.state_id);

            for trans in state_trans {
                // 检查字符匹配
                if trans.code_min <= prev_c && trans.code_max >= prev_c {
                    // 检查断言和字符类
                    if trans.assertions != 0 {
                        if check_assertions(trans.assertions, pos, prev_c, next_c, eflags, tnfa) {
                            continue;
                        }
                        if check_char_classes(trans, prev_c, tnfa) {
                            continue;
                        }
                    }

                    // 计算转移后的标签
                    if num_tags > 0 {
                        let n = tmp_tags.len().min(rs.tags.len());
                        tmp_tags[..n].copy_from_slice(&rs.tags[..n]);
                    }
                    if let Some(ref trans_tags) = trans.tags {
                        for &t in trans_tags.iter() {
                            if t >= 0 && (t as usize) < num_tags {
                                tmp_tags[t as usize] = pos;
                            }
                        }
                    }

                    let dest = trans.state_id;
                    if dest >= 0 && (dest as usize) < num_states {
                        if reach_pos[dest as usize].pos < pos {
                            // 未访问的状态
                            let mut new_tags: Box<[regoff_t]> =
                                vec![-1i64; num_tags].into_boxed_slice();
                            if num_tags > 0 {
                                let n2 = new_tags.len().min(tmp_tags.len());
                                new_tags[..n2].copy_from_slice(&tmp_tags[..n2]);
                            }

                            if dest == tnfa.final_id
                                && (match_eo == -1
                                    || (num_tags > 0
                                        && match_tags.as_ref().map_or(true, |mt| {
                                            new_tags[0] <= mt[0]
                                        })))
                            {
                                match_eo = pos;
                                new_match = true;
                                if let Some(ref mut mt) = match_tags {
                                    { let n3 = mt.len().min(new_tags.len()); mt[..n3].copy_from_slice(&new_tags[..n3]); }
                                }
                            }

                            reach_pos[dest as usize].pos = pos;
                            reach_pos[dest as usize].tags = Some(new_tags.clone());
                            reach_next.push(ReachState { state_id: dest, tags: new_tags });
                        } else if num_tags > 0 {
                            // 已访问：检查标签是否更优
                            if let Some(ref existing_tags) = reach_pos[dest as usize].tags {
                                if tre_tag_order(&tnfa.tag_directions, &tmp_tags, existing_tags) {
                                    let mut better_tags: Box<[regoff_t]> =
                                        vec![-1i64; num_tags].into_boxed_slice();
                                    better_tags.copy_from_slice(&tmp_tags);

                                    if dest == tnfa.final_id
                                        && (match_eo == -1
                                            || (match_tags.as_ref().map_or(true, |mt| {
                                                better_tags[0] <= mt[0]
                                            })))
                                    {
                                        match_eo = pos;
                                        new_match = true;
                                        if let Some(ref mut mt) = match_tags {
                                            { let n4 = mt.len().min(better_tags.len()); mt[..n4].copy_from_slice(&better_tags[..n4]); }
                                        }
                                    }

                                    reach_pos[dest as usize].tags = Some(better_tags.clone());
                                    for rn in reach_next.iter_mut() {
                                        if rn.state_id == dest {
                                            rn.tags = better_tags;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    *match_end_ofs = match_eo;
    if match_eo >= 0 {
        RegError::Ok
    } else {
        RegError::NoMatch
    }
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use super::super::tre::{
        TnfaTransition, SubmatchData, ASSERT_AT_BOL, ASSERT_AT_EOL,
    };

    // ---- ReachState 测试 ----

    test!("test_reach_state_creation" {
        let tags: Box<[regoff_t]> = Box::new([0, -1, -1]);
        let state = ReachState {
            state_id: 1,
            tags,
        };
        assert_eq!(state.state_id, 1);
        assert_eq!(state.tags.len(), 3);
        assert_eq!(state.tags[0], 0);
        assert_eq!(state.tags[1], -1);
    });

    test!("test_reach_state_clone" {
        let tags: Box<[regoff_t]> = Box::new([5, 10]);
        let state = ReachState {
            state_id: 2,
            tags,
        };
        let cloned = state.clone();
        assert_eq!(cloned.state_id, 2);
        assert_eq!(cloned.tags.len(), 2);
    });

    // ---- ReachPos 测试 ----

    test!("test_reach_pos_creation" {
        let rp = ReachPos {
            pos: 0,
            tags: None,
        };
        assert_eq!(rp.pos, 0);
        assert!(rp.tags.is_none());
    });

    test!("test_reach_pos_with_tags" {
        let tags: Box<[regoff_t]> = Box::new([1, 2, -1]);
        let rp = ReachPos {
            pos: 3,
            tags: Some(tags),
        };
        assert_eq!(rp.pos, 3);
        assert!(rp.tags.is_some());
        assert_eq!(rp.tags.as_ref().unwrap().len(), 3);
    });

    // ---- get_state_transitions 测试 ----

    test!("test_get_state_transitions_empty" {
        let transitions: &[TnfaTransition] = &[];
        let result = get_state_transitions(transitions, 0);
        assert!(result.is_empty());
    });

    test!("test_get_state_transitions_state_0" {
        let transitions: &[TnfaTransition] = &[
            TnfaTransition {
                code_min: b'a' as TreCint,
                code_max: b'a' as TreCint,
                state_id: 1,
                assertions: 0,
                tags: None,
                u_class: None,
                u_backref: None,
                neg_classes: None,
            },
            TnfaTransition {
                code_min: 0,
                code_max: 0,
                state_id: END_STATE_MARKER,
                assertions: 0,
                tags: None,
                u_class: None,
                u_backref: None,
                neg_classes: None,
            },
            TnfaTransition {
                code_min: b'b' as TreCint,
                code_max: b'b' as TreCint,
                state_id: 2,
                assertions: 0,
                tags: None,
                u_class: None,
                u_backref: None,
                neg_classes: None,
            },
            TnfaTransition {
                code_min: 0,
                code_max: 0,
                state_id: END_STATE_MARKER,
                assertions: 0,
                tags: None,
                u_class: None,
                u_backref: None,
                neg_classes: None,
            },
        ];
        let result = get_state_transitions(transitions, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].state_id, 1);
        assert_eq!(result[0].code_min, b'a' as TreCint);
    });

    // ---- check_assertions 测试 ----

    fn make_empty_tnfa() -> Tnfa {
        Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 1,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
            num_states: 1,
            cflags: 0,
            have_backrefs: false,
            have_approx: false,
        }
    }

    test!("test_check_assertions_no_assertions" {
        let tnfa = make_empty_tnfa();
        assert!(!check_assertions(0, 0, 0, b'a' as TreCint, 0, &tnfa));
    });

    test!("test_check_assertions_bol_not_at_beginning" {
        let tnfa = make_empty_tnfa();
        // ASSERT_AT_BOL 在 pos > 0 时应失败
        assert!(check_assertions(ASSERT_AT_BOL, 1, 0, b'a' as TreCint, 0, &tnfa));
    });

    test!("test_check_assertions_bol_at_beginning" {
        let tnfa = make_empty_tnfa();
        // ASSERT_AT_BOL 在 pos == 0 且非 REG_NOTBOL 时应通过
        assert!(!check_assertions(ASSERT_AT_BOL, 0, 0, b'a' as TreCint, 0, &tnfa));
    });

    test!("test_check_assertions_eol_at_end" {
        let tnfa = make_empty_tnfa();
        // ASSERT_AT_EOL 在 next_c == 0 且非 REG_NOTEOL 时应通过
        assert!(!check_assertions(ASSERT_AT_EOL, 3, b'a' as TreCint, 0, 0, &tnfa));
    });

    test!("test_check_assertions_eol_not_at_end" {
        let tnfa = make_empty_tnfa();
        // ASSERT_AT_EOL 在 next_c != 0 时应失败
        assert!(check_assertions(ASSERT_AT_EOL, 3, b'a' as TreCint, b'b' as TreCint, 0, &tnfa));
    });

    // ---- tnfa_run_parallel 测试 ----

    fn make_simple_tnfa() -> Tnfa {
        Tnfa {
            transitions: Box::new([
                // 状态 0: 从初始态接受 'a' 到状态 1
                TnfaTransition {
                    code_min: b'a' as TreCint,
                    code_max: b'a' as TreCint,
                    state_id: 1,
                    assertions: 0,
                    tags: None,
                    u_class: None,
                    u_backref: None,
                    neg_classes: None,
                },
                // 状态 0 终止标记
                TnfaTransition {
                    code_min: 0,
                    code_max: 0,
                    state_id: END_STATE_MARKER,
                    assertions: 0,
                    tags: None,
                    u_class: None,
                    u_backref: None,
                    neg_classes: None,
                },
                // 状态 1: 从状态 1 接受 'b' 到终态 2
                TnfaTransition {
                    code_min: b'b' as TreCint,
                    code_max: b'b' as TreCint,
                    state_id: 2,
                    assertions: 0,
                    tags: None,
                    u_class: None,
                    u_backref: None,
                    neg_classes: None,
                },
                // 状态 1 终止标记
                TnfaTransition {
                    code_min: 0,
                    code_max: 0,
                    state_id: END_STATE_MARKER,
                    assertions: 0,
                    tags: None,
                    u_class: None,
                    u_backref: None,
                    neg_classes: None,
                },
                // 终态 2: 无出边
                TnfaTransition {
                    code_min: 0,
                    code_max: 0,
                    state_id: END_STATE_MARKER,
                    assertions: 0,
                    tags: None,
                    u_class: None,
                    u_backref: None,
                    neg_classes: None,
                },
            ]),
            initial_id: 0,
            final_id: 2,
            initial_tags: None,
            submatch_data: Box::new([SubmatchData {
                so_tag: -1,
                eo_tag: -1,
                parents: None,
            }]),
            firstpos_chars: [0u8; 32],
            first_char: b'a' as i32,
            num_submatches: 1,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
            num_states: 3,
            cflags: 0,
            have_backrefs: false,
            have_approx: false,
        }
    }

    test!("test_parallel_simple_match" {
        let tnfa = make_simple_tnfa();
        let string: &[u8] = b"ab";
        let mut match_eo: regoff_t = -1;
        let result = tnfa_run_parallel(
            &tnfa,
            string,
            None,
            0,
            &mut match_eo,
        );
        // 实现后：应返回 Ok 且 match_eo == 2
        assert_eq!(result, RegError::Ok);
        assert_eq!(match_eo, 2);
    });

    test!("test_parallel_no_match" {
        let tnfa = make_simple_tnfa();
        let string: &[u8] = b"ac"; // 'c' 不匹配 'b'
        let mut match_eo: regoff_t = -1;
        let result = tnfa_run_parallel(
            &tnfa,
            string,
            None,
            0,
            &mut match_eo,
        );
        // 实现后：应返回 NoMatch
        assert_eq!(result, RegError::NoMatch);
    });

    test!("test_parallel_empty_string" {
        let tnfa = make_simple_tnfa();
        let string: &[u8] = b"";
        let mut match_eo: regoff_t = -1;
        let result = tnfa_run_parallel(
            &tnfa,
            string,
            None,
            0,
            &mut match_eo,
        );
        // 空字符串不匹配 "ab"
        assert_eq!(result, RegError::NoMatch);
    });

    test!("test_parallel_with_tags" {
        let tnfa = make_simple_tnfa();
        let string: &[u8] = b"ab";
        let mut tags_buf: [regoff_t; 4] = [-1; 4];
        let mut match_eo: regoff_t = -1;
        let result = tnfa_run_parallel(
            &tnfa,
            string,
            Some(&mut tags_buf),
            0,
            &mut match_eo,
        );
        assert_eq!(result, RegError::Ok);
        assert_eq!(match_eo, 2);
    });

    test!("test_parallel_eflags_notbol" {
        let tnfa = make_simple_tnfa();
        let string: &[u8] = b"ab";
        let mut match_eo: regoff_t = -1;
        let result = tnfa_run_parallel(
            &tnfa,
            string,
            None,
            REG_NOTBOL,
            &mut match_eo,
        );
        // NOTBOL 不应影响简单匹配
        assert_eq!(result, RegError::Ok);
    });
}
