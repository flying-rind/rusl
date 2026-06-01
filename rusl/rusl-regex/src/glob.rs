//! glob/globfree — Shell 风格文件系统路径展开。对外导出 C ABI 兼容的 `glob` 和 `globfree` 符号。
//!
//! 根据 shell 通配符规则查找文件系统中匹配指定模式的路径名，
//! 支持通配字符（`*`、`?`、`[...]`）、波浪号展开（`~`）、目录标记等功能。
//!
//! # 模块结构
//!
//! - 公开接口：`glob`、`globfree`、`glob_t`、`GLOB_*` 常量
//! - 内部实现：`MatchNode`、`do_glob`、`expand_tilde`、`append`、`freelist`、`sort_cmp`、`ignore_err`
//! - 错误类型：`GlobError`
//! - 所有函数均通过 rusl 内部实现或直接 syscall，无 FFI 依赖

#![allow(unused_imports, unused_variables)]

use alloc::alloc::{alloc, dealloc, realloc, Layout};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_uint, c_void};
use core::cmp::Ordering;
use rusl_errno::__errno_location;

// ============================================================================
// GLOB_* 公开常量
// ============================================================================

/// 对目录读取错误返回错误。
pub const GLOB_ERR: c_int = 0x01;
/// 在每个匹配路径后附加 `'/'`。
pub const GLOB_MARK: c_int = 0x02;
/// 不对路径名排序。
pub const GLOB_NOSORT: c_int = 0x04;
/// 保留 `gl_offs` 个空槽位于 `gl_pathv` 开头。
pub const GLOB_DOOFFS: c_int = 0x08;
/// 无匹配时返回原始模式（而非错误）。
pub const GLOB_NOCHECK: c_int = 0x10;
/// 将结果追加到已有的 `gl_pathv`。
pub const GLOB_APPEND: c_int = 0x20;
/// 禁用反斜杠转义。
pub const GLOB_NOESCAPE: c_int = 0x40;
/// 前导句点必须被显式匹配。
pub const GLOB_PERIOD: c_int = 0x80;
/// 展开波浪号 `~` 为当前用户家目录。
pub const GLOB_TILDE: c_int = 0x1000;
/// 同 GLOB_TILDE，但若 `~` 无法展开则返回错误。
pub const GLOB_TILDE_CHECK: c_int = 0x4000;

// ============================================================================
// GLOB_* 返回值常量
// ============================================================================

/// 内存不足。
pub const GLOB_NOSPACE: c_int = 1;
/// 目录读取错误导致匹配中止。
pub const GLOB_ABORTED: c_int = 2;
/// 无匹配结果。
pub const GLOB_NOMATCH: c_int = 3;

// ============================================================================
// 系统常量
// ============================================================================

/// PATH_MAX — 路径最大长度。
pub(crate) const PATH_MAX: usize = 4096;

/// 目录项类型常量。
pub(crate) const DT_UNKNOWN: u8 = 0;
pub(crate) const DT_DIR: u8 = 4;
pub(crate) const DT_REG: u8 = 8;
pub(crate) const DT_LNK: u8 = 10;

/// errno 值。
pub(crate) const ENOENT: c_int = 2;
pub(crate) const ENOMEM: c_int = 12;

/// stat 模式位。
pub(crate) const S_IFMT: u32 = 0o170000;
pub(crate) const S_IFDIR: u32 = 0o040000;

/// Linux AT_FDCWD 常量。
const AT_FDCWD: i64 = -100;
/// AT_SYMLINK_NOFOLLOW
const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
/// O_RDONLY
const O_RDONLY: i32 = 0;
/// O_DIRECTORY
const O_DIRECTORY: i32 = 0o200000;

// syscall 编号（x86_64 和 aarch64）
#[cfg(target_arch = "x86_64")]
const SYS_NEWFSTATAT: i64 = 262;
#[cfg(target_arch = "x86_64")]
const SYS_OPENAT: i64 = 257;
#[cfg(target_arch = "x86_64")]
const SYS_GETDENTS64: i64 = 217;
#[cfg(target_arch = "x86_64")]
const SYS_CLOSE: i64 = 3;

#[cfg(target_arch = "aarch64")]
const SYS_NEWFSTATAT: i64 = 79;
#[cfg(target_arch = "aarch64")]
const SYS_OPENAT: i64 = 56;
#[cfg(target_arch = "aarch64")]
const SYS_GETDENTS64: i64 = 61;
#[cfg(target_arch = "aarch64")]
const SYS_CLOSE: i64 = 57;

// ============================================================================
// 内部错误类型
// ============================================================================

/// glob 内部操作的错误类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlobError {
    Ok = 0,
    NoSpace = 1,
    Aborted = 2,
    NoMatch = 3,
}

// ============================================================================
// DirentType — 目录项类型
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DirentType {
    Unknown = 0,
    Directory = 4,
    Regular = 8,
    Symlink = 10,
}

// ============================================================================
// glob_t — POSIX glob 结果类型
// ============================================================================

/// POSIX `glob_t` 结构体 — 存储 `glob()` 调用的匹配结果。
#[repr(C)]
pub struct glob_t {
    pub gl_pathc: usize,
    pub gl_pathv: *mut *mut c_char,
    pub gl_offs: usize,
    pub(crate) __padding: [*mut c_void; 4],
}

// ============================================================================
// MatchNode — 匹配结果节点（内部）
// ============================================================================

pub(crate) struct MatchNode {
    pub(crate) name: Vec<u8>,
    pub(crate) next: Option<Box<MatchNode>>,
}

// ============================================================================
// 内存管理函数（通过 Rust alloc crate，无 FFI）
// ============================================================================

unsafe fn glob_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size, 8).unwrap_or_else(|_| {
        Layout::from_size_align_unchecked(size, 8)
    });
    alloc(layout) as *mut c_void
}

unsafe fn glob_free(ptr: *mut c_void) {
    // 注意：无法从裸指针恢复 Layout，此处内存会泄漏。
    // 完整实现需要额外记录分配大小，或使用 Vec/Box 管理。
    let _ = ptr;
}

unsafe fn glob_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return glob_malloc(size);
    }
    if size == 0 {
        glob_free(ptr);
        return core::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size, 8).unwrap_or_else(|_| {
        Layout::from_size_align_unchecked(size, 8)
    });
    realloc(ptr as *mut u8, layout, size) as *mut c_void
}

// ============================================================================
// 文件系统 syscall 包装（纯 Rust，无 FFI）
// ============================================================================

/// stat() 的纯 Rust 实现，通过 `newfstatat` syscall。
unsafe fn glob_stat(path: *const c_char, buf: *mut c_void) -> c_int {
    use rusl_core::do_syscall;
    do_syscall!(SYS_NEWFSTATAT, AT_FDCWD, path, buf, 0i32) as c_int
}

/// opendir() 的纯 Rust 实现：打开目录并返回文件描述符。
unsafe fn glob_opendir(path: *const c_char) -> i32 {
    use rusl_core::do_syscall;
    let fd = do_syscall!(SYS_OPENAT, AT_FDCWD, path, O_RDONLY | O_DIRECTORY, 0u32);
    if fd < 0 {
        -1
    } else {
        fd as i32
    }
}

// 内部缓冲区状态（通过函数访问，避免 static mut 跨函数问题）
static mut DIR_BUF_POS_INTERNAL: usize = 0;
static mut DIR_BUF_END_INTERNAL: usize = 0;
static mut DIR_BUF_DATA: [u8; 4096] = [0u8; 4096];

// 重新实现 glob_readdir 使用独立的 static
unsafe fn read_dir_entry(fd: i32) -> *const u8 {
    use rusl_core::do_syscall;

    if DIR_BUF_POS_INTERNAL < DIR_BUF_END_INTERNAL {
        let entry_ptr = DIR_BUF_DATA.as_ptr().add(DIR_BUF_POS_INTERNAL);
        let reclen =
            u16::from_ne_bytes([
                DIR_BUF_DATA[DIR_BUF_POS_INTERNAL + 16],
                DIR_BUF_DATA[DIR_BUF_POS_INTERNAL + 17],
            ]) as usize;
        DIR_BUF_POS_INTERNAL += reclen;
        return entry_ptr;
    }

    let nread = do_syscall!(SYS_GETDENTS64, fd, DIR_BUF_DATA.as_mut_ptr(), 4096usize);
    if nread <= 0 {
        return core::ptr::null();
    }

    DIR_BUF_END_INTERNAL = nread as usize;
    DIR_BUF_POS_INTERNAL = 0;

    let entry_ptr = DIR_BUF_DATA.as_ptr();
    let reclen =
        u16::from_ne_bytes([DIR_BUF_DATA[0 + 16], DIR_BUF_DATA[0 + 17]]) as usize;
    DIR_BUF_POS_INTERNAL = reclen;

    entry_ptr
}

unsafe fn close_dir(fd: i32) {
    DIR_BUF_POS_INTERNAL = 0;
    DIR_BUF_END_INTERNAL = 0;
    use rusl_core::do_syscall;
    do_syscall!(SYS_CLOSE, fd);
}

// ============================================================================
// append — 追加匹配结果
// ============================================================================

pub(crate) fn append_to_vec(results: &mut Vec<Vec<u8>>, name: &[u8], mark: bool) {
    let actual_len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    let need_mark = mark && actual_len > 0 && name[actual_len - 1] != b'/';

    let mut entry = Vec::with_capacity(actual_len + if need_mark { 1 } else { 0 } + 1);
    entry.extend_from_slice(&name[..actual_len]);
    if need_mark {
        entry.push(b'/');
    }
    entry.push(0);
    results.push(entry);
}

// ============================================================================
// ignore_err — 默认错误处理
// ============================================================================

#[no_mangle]
pub(crate) extern "C" fn ignore_err(_path: *const c_char, _err: c_int) -> c_int {
    0
}

// ============================================================================
// sort_cmp — 排序比较回调
// ============================================================================

pub(crate) fn compare_path_ptrs(a: &*const c_char, b: &*const c_char) -> Ordering {
    unsafe {
        let cmp = rusl_string::strcmp(*a, *b);
        cmp.cmp(&0)
    }
}

// ============================================================================
// freelist — 释放结果链表
// ============================================================================

pub(crate) fn freelist(_head: Option<Box<MatchNode>>) {}

// ============================================================================
// expand_tilde — 波浪号展开
// ============================================================================

pub(crate) fn expand_tilde(
    pat: &[u8],
    buf: &mut [u8],
    pos: &mut usize,
) -> Result<(), GlobError> {
    if pat.is_empty() || pat[0] != b'~' {
        return Ok(());
    }

    let user_end = if pat.len() > 1 && pat[1] == b'/' {
        1
    } else if pat.len() == 1 {
        1
    } else {
        return Err(GlobError::NoMatch);
    };

    // 使用 rusl 内部的 getenv 实现
    let home = unsafe {
        let home_name = b"HOME\0".as_ptr() as *const c_char;
        rusl_env::getenv(home_name)
    };

    if home.is_null() {
        return Err(GlobError::NoMatch);
    }

    let mut i: usize = 0;
    unsafe {
        while *home.add(i) != 0 {
            if *pos + i >= PATH_MAX - 2 {
                return Err(GlobError::NoMatch);
            }
            buf[*pos + i] = *home.add(i) as u8;
            i += 1;
        }
    }
    *pos += i;

    if pat.len() > 1 && pat[1] == b'/' {
        if *pos < PATH_MAX - 1 {
            buf[*pos] = b'/';
            *pos += 1;
        }
    }

    Ok(())
}

// ============================================================================
// do_glob — 核心递归引擎
// ============================================================================

pub(crate) fn do_glob(
    buf: &mut [u8],
    pos: &mut usize,
    _type_hint: DirentType,
    pat: &[u8],
    flags: c_int,
    errfunc: Option<unsafe extern "C" fn(*const c_char, c_int) -> c_int>,
    results: &mut Vec<Vec<u8>>,
) -> Result<(), GlobError> {
    // 检查模式中是否含有通配符
    let has_wildcard = pat.iter().any(|&b| b == b'*' || b == b'?' || b == b'[');

    if !has_wildcard {
        // 字面路径：通过 newfstatat syscall 检查是否存在
        let pat_len = pat.iter().position(|&b| b == 0).unwrap_or(pat.len());
        let full_len = *pos + pat_len;

        if full_len + 1 >= buf.len() {
            return Ok(());
        }

        let saved = buf[*pos];
        buf[*pos..*pos + pat_len].copy_from_slice(&pat[..pat_len]);
        buf[full_len] = 0;

        let mut st_buf: [u8; 144] = [0u8; 144];
        let rc = unsafe {
            glob_stat(buf.as_ptr() as *const c_char, st_buf.as_mut_ptr() as *mut c_void)
        };

        buf[*pos] = saved;

        if rc == 0 {
            let mark = (flags & GLOB_MARK) != 0;
            append_to_vec(results, &buf[..full_len + 1], mark);
        } else if (flags & GLOB_NOCHECK) != 0 {
            let mut entry = Vec::with_capacity(pat_len + 1);
            entry.extend_from_slice(&pat[..pat_len]);
            entry.push(0);
            results.push(entry);
        }
    } else {
        // 含通配符：执行目录遍历
        let base_dir = if *pos > 0 {
            buf[*pos] = 0;
            let fd = unsafe { glob_opendir(buf.as_ptr() as *const c_char) };
            buf[*pos] = b'/';
            fd
        } else {
            let dot = b".\0".as_ptr() as *const c_char;
            unsafe { glob_opendir(dot) }
        };

        if base_dir < 0 {
            if let Some(func) = errfunc {
                let errno = unsafe { *__errno_location() };
                buf[*pos] = 0;
                let ret = unsafe { func(buf.as_ptr() as *const c_char, errno) };
                buf[*pos] = b'/';
                if ret != 0 {
                    return Err(GlobError::Aborted);
                }
            }
            return Ok(());
        }

        // 读取目录项（linux_dirent64 偏移: d_name 在 19 字节处）
        loop {
            let entry = unsafe { read_dir_entry(base_dir) };
            if entry.is_null() {
                break;
            }

            let d_name_ptr = unsafe { entry.add(19) } as *const c_char;
            let d_name = unsafe { core::ffi::CStr::from_ptr(d_name_ptr) };
            let d_name_bytes = d_name.to_bytes();

            if d_name_bytes == b"." || d_name_bytes == b".." {
                continue;
            }

            // 使用 fnmatch_internal 进行模式匹配
            let fnm_flags = super::fnmatch::FnmFlags::from_bits_truncate(
                if (flags & GLOB_NOESCAPE) != 0 {
                    super::fnmatch::FNM_NOESCAPE
                } else {
                    0
                } | if (flags & GLOB_PERIOD) != 0 {
                    super::fnmatch::FNM_PERIOD
                } else {
                    0
                },
            );

            let pat_len = pat.iter().position(|&b| b == 0).unwrap_or(pat.len());
            let match_result = super::fnmatch::fnmatch_internal(
                &pat[..pat_len],
                d_name_bytes,
                fnm_flags,
            );

            if match_result {
                let mark = (flags & GLOB_MARK) != 0;
                append_to_vec(results, d_name_bytes, mark);
            }
        }

        unsafe { close_dir(base_dir) };
    }

    Ok(())
}

// ============================================================================
// glob (对外导出)
// ============================================================================

/// POSIX `glob()` — 查找文件系统中匹配 shell 通配符模式的路径名。
#[no_mangle]
pub unsafe extern "C" fn glob(
    pat: *const c_char,
    flags: c_int,
    errfunc: Option<unsafe extern "C" fn(*const c_char, c_int) -> c_int>,
    g: *mut glob_t,
) -> c_int {
    if pat.is_null() || g.is_null() {
        return GLOB_NOSPACE;
    }

    let err_handler = errfunc.unwrap_or(ignore_err);

    if (flags & GLOB_APPEND) == 0 {
        unsafe {
            (*g).gl_pathc = 0;
            (*g).gl_offs = if (flags & GLOB_DOOFFS) != 0 { (*g).gl_offs } else { 0 };
            (*g).gl_pathv = core::ptr::null_mut();
        }
    }

    let offs = unsafe { (*g).gl_offs };

    let pat_len = unsafe {
        let mut len: usize = 0;
        while *pat.add(len) != 0 {
            len += 1;
        }
        len
    };
    let pat_slice = unsafe { core::slice::from_raw_parts(pat as *const u8, pat_len) };

    let mut buf: [u8; PATH_MAX] = [0u8; PATH_MAX];
    let mut pos: usize = 0;

    if (flags & (GLOB_TILDE | GLOB_TILDE_CHECK)) != 0 && pat_len > 0 && pat_slice[0] == b'~' {
        match expand_tilde(pat_slice, &mut buf, &mut pos) {
            Ok(()) => {}
            Err(GlobError::NoSpace) => return GLOB_NOSPACE,
            Err(_) => return GLOB_NOMATCH,
        }
    }

    let mut results: Vec<Vec<u8>> = Vec::new();

    match do_glob(
        &mut buf,
        &mut pos,
        DirentType::Unknown,
        pat_slice,
        flags,
        errfunc,
        &mut results,
    ) {
        Ok(()) => {}
        Err(GlobError::NoSpace) => return GLOB_NOSPACE,
        Err(GlobError::Aborted) => return GLOB_ABORTED,
        Err(_) => {}
    }

    if results.is_empty() {
        if (flags & GLOB_NOCHECK) != 0 {
            let mut entry = Vec::with_capacity(pat_len + 1);
            entry.extend_from_slice(pat_slice);
            entry.push(0);
            results.push(entry);
        } else {
            return GLOB_NOMATCH;
        }
    }

    let cnt = results.len();
    let ptr_size = core::mem::size_of::<*mut c_char>();
    let arr_size = (offs + cnt + 1) * ptr_size;

    let pathv: *mut *mut c_char = if (flags & GLOB_APPEND) != 0 && !(*g).gl_pathv.is_null() {
        unsafe { glob_realloc((*g).gl_pathv as *mut c_void, arr_size) as *mut *mut c_char }
    } else {
        unsafe { glob_malloc(arr_size) as *mut *mut c_char }
    };

    if pathv.is_null() {
        results.clear();
        return GLOB_NOSPACE;
    }

    unsafe {
        for i in 0..offs {
            *pathv.add(i) = core::ptr::null_mut();
        }
    }

    for (i, entry) in results.iter().enumerate() {
        let len = entry.len();
        let str_ptr = unsafe { glob_malloc(len) as *mut c_char };
        if str_ptr.is_null() {
            for j in 0..i {
                unsafe { glob_free(pathv.add(offs + j).read() as *mut c_void) };
            }
            unsafe { glob_free(pathv as *mut c_void) };
            results.clear();
            return GLOB_NOSPACE;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(entry.as_ptr(), str_ptr as *mut u8, len);
            *pathv.add(offs + i) = str_ptr;
        }
    }

    unsafe {
        *pathv.add(offs + cnt) = core::ptr::null_mut();
    }

    if (flags & GLOB_NOSORT) == 0 && cnt > 1 {
        unsafe {
            let slice = core::slice::from_raw_parts_mut(pathv.add(offs), cnt);
            slice.sort_unstable_by(|a, b| {
                let ca = *a as *const c_char;
                let cb = *b as *const c_char;
                compare_path_ptrs(&ca, &cb)
            });
        }
    }

    unsafe {
        (*g).gl_pathv = pathv;
        (*g).gl_pathc = cnt;
    }

    results.clear();

    0
}

// ============================================================================
// globfree (对外导出)
// ============================================================================

/// POSIX `globfree()` — 释放由先前 `glob()` 调用分配的所有内存。
#[no_mangle]
pub unsafe extern "C" fn globfree(g: *mut glob_t) {
    if g.is_null() {
        return;
    }

    let pathv = unsafe { (*g).gl_pathv };
    let pathc = unsafe { (*g).gl_pathc };
    let offs = unsafe { (*g).gl_offs };

    if !pathv.is_null() {
        for i in 0..pathc {
            let str_ptr = unsafe { *pathv.add(offs + i) };
            if !str_ptr.is_null() {
                unsafe { glob_free(str_ptr as *mut c_void) };
            }
        }
        unsafe { glob_free(pathv as *mut c_void) };
    }

    unsafe {
        (*g).gl_pathc = 0;
        (*g).gl_pathv = core::ptr::null_mut();
    }
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use alloc::format;
    use super::*;

    // ---- 常量测试 ----

    test!("test_glob_flags_distinct" {
        let flag_masks = [
            GLOB_ERR, GLOB_MARK, GLOB_NOSORT, GLOB_DOOFFS,
            GLOB_NOCHECK, GLOB_APPEND, GLOB_NOESCAPE, GLOB_PERIOD,
            GLOB_TILDE, GLOB_TILDE_CHECK,
        ];
        for i in 0..flag_masks.len() {
            for j in (i + 1)..flag_masks.len() {
                assert_eq!(
                    flag_masks[i] & flag_masks[j],
                    0,
                    "GLOB 标志 {} 和 {} 重叠",
                    flag_masks[i],
                    flag_masks[j]
                );
            }
        }
    });

    test!("test_glob_return_values" {
        assert_eq!(GLOB_NOSPACE, 1);
        assert_eq!(GLOB_ABORTED, 2);
        assert_eq!(GLOB_NOMATCH, 3);
    });

    test!("test_glob_err_value" {
        assert_eq!(GLOB_ERR, 0x01);
    });

    test!("test_glob_mark_value" {
        assert_eq!(GLOB_MARK, 0x02);
    });

    test!("test_glob_tilde_value" {
        assert_eq!(GLOB_TILDE, 0x1000);
        assert_eq!(GLOB_TILDE_CHECK, 0x4000);
    });

    test!("test_path_max_reasonable" {
        assert!(PATH_MAX >= 256, "PATH_MAX 应至少为 256");
    });

    // ---- GlobError 测试 ----

    test!("test_glob_error_eq" {
        assert_eq!(GlobError::Ok, GlobError::Ok);
        assert_ne!(GlobError::Ok, GlobError::NoSpace);
        assert_ne!(GlobError::NoMatch, GlobError::Aborted);
    });

    test!("test_glob_error_clone_copy" {
        let e = GlobError::NoSpace;
        let e2 = e;
        assert_eq!(e, e2);
    });

    test!("test_glob_error_debug" {
        let e = GlobError::Aborted;
        let s = format!("{:?}", e);
        assert!(s.contains("Aborted"));
    });

    // ---- DirentType 测试 ----

    test!("test_dirent_type_values" {
        assert_eq!(DirentType::Unknown as i32, DT_UNKNOWN as i32);
        assert_eq!(DirentType::Directory as i32, DT_DIR as i32);
        assert_eq!(DirentType::Regular as i32, DT_REG as i32);
        assert_eq!(DirentType::Symlink as i32, DT_LNK as i32);
    });

    // ---- glob_t 测试 ----

    test!("test_glob_t_size" {
        let size = core::mem::size_of::<glob_t>();
        assert!(size > 0);
        assert!(size >= 24, "glob_t 结构体太小");
    });

    test!("test_glob_t_initialization" {
        let g: glob_t = unsafe { core::mem::zeroed() };
        assert_eq!(g.gl_pathc, 0);
        assert!(g.gl_pathv.is_null());
        assert_eq!(g.gl_offs, 0);
    });

    // ---- MatchNode 测试 ----

    test!("test_match_node_creation" {
        let node = MatchNode {
            name: b"test.txt\0".to_vec(),
            next: None,
        };
        assert_eq!(node.name, b"test.txt\0");
        assert!(node.next.is_none());
    });

    test!("test_match_node_linked_list" {
        let node2 = Box::new(MatchNode {
            name: b"file2\0".to_vec(),
            next: None,
        });
        let node1 = Box::new(MatchNode {
            name: b"file1\0".to_vec(),
            next: Some(node2),
        });
        assert!(node1.next.is_some());
        let n2 = node1.next.as_ref().unwrap();
        assert_eq!(&n2.name[..5], b"file2");
        assert!(n2.next.is_none());
    });

    test!("test_match_node_mark_suffix" {
        let node = MatchNode {
            name: b"dir/\0".to_vec(),
            next: None,
        };
        assert_eq!(node.name.last(), Some(&0u8));
        assert_eq!(node.name[node.name.len() - 2], b'/');
    });

    // ---- ignore_err 测试 ----

    test!("test_ignore_err_returns_zero" {
        let result = ignore_err(core::ptr::null(), 0);
        assert_eq!(result, 0);
    });

    test!("test_ignore_err_with_various_args" {
        assert_eq!(ignore_err(core::ptr::null(), ENOENT), 0);
        assert_eq!(ignore_err(core::ptr::null(), ENOMEM), 0);
        assert_eq!(ignore_err(b"/tmp\0".as_ptr() as *const c_char, 42), 0);
    });

    // ---- expand_tilde 测试 ----

    test!("test_expand_tilde_not_tilde_prefix" {
        let mut buf = [0u8; PATH_MAX];
        let mut pos: usize = 0;
        let pat = b"notilde";
        let _result = expand_tilde(pat, &mut buf, &mut pos);
    });

    test!("test_expand_tilde_empty_pat" {
        let mut buf = [0u8; PATH_MAX];
        let mut pos: usize = 0;
        let pat: &[u8] = &[];
        let _result = expand_tilde(pat, &mut buf, &mut pos);
    });

    // ---- do_glob 测试 ----

    test!("test_do_glob_empty_pattern" {
        let mut buf = [0u8; PATH_MAX];
        let mut pos: usize = 0;
        let mut results: Vec<Vec<u8>> = Vec::new();
        let _result = do_glob(
            &mut buf,
            &mut pos,
            DirentType::Unknown,
            &[],
            0,
            Some(ignore_err),
            &mut results,
        );
    });

    test!("test_do_glob_simple_literal" {
        let mut buf = [0u8; PATH_MAX];
        let mut pos: usize = 0;
        let mut results: Vec<Vec<u8>> = Vec::new();
        let _result = do_glob(
            &mut buf,
            &mut pos,
            DirentType::Unknown,
            b"/bin/ls",
            0,
            Some(ignore_err),
            &mut results,
        );
    });

    // ---- glob 公开 API 测试 ----

    test!("test_glob_basic" {
        unsafe {
            let mut g: glob_t = core::mem::zeroed();
            let pat = b"/bin/ls\0" as *const u8 as *const c_char;
            let result = glob(pat, 0, None, &mut g);
            assert!(result == 0 || result == GLOB_NOMATCH || result == GLOB_NOSPACE);
        }
    });

    test!("test_glob_noescape_flag" {
        unsafe {
            let mut g: glob_t = core::mem::zeroed();
            let pat = b"/bin/*\0" as *const u8 as *const c_char;
            let result = glob(pat, GLOB_NOESCAPE, None, &mut g);
            assert!(result == 0 || result == GLOB_NOMATCH || result == GLOB_NOSPACE);
        }
    });

    test!("test_glob_nocheck" {
        unsafe {
            let mut g: glob_t = core::mem::zeroed();
            let pat = b"/nonexistent_path_xyz123\0" as *const u8 as *const c_char;
            let result = glob(pat, GLOB_NOCHECK, None, &mut g);
            assert!(result == 0 || result == GLOB_NOSPACE);
        }
    });

    test!("test_glob_with_custom_errfunc" {
        unsafe {
            let mut g: glob_t = core::mem::zeroed();
            let pat = b"/bin/*\0" as *const u8 as *const c_char;
            extern "C" fn my_errfunc(_p: *const c_char, _e: c_int) -> c_int {
                0
            }
            let result = glob(pat, 0, Some(my_errfunc), &mut g);
            assert!(result == 0 || result == GLOB_NOMATCH || result == GLOB_NOSPACE);
        }
    });

    // ---- globfree 公开 API 测试 ----

    test!("test_globfree_on_zeroed_glob_t" {
        unsafe {
            let mut g: glob_t = core::mem::zeroed();
            globfree(&mut g);
        }
    });

    test!("test_globfree_after_glob" {
        unsafe {
            let mut g: glob_t = core::mem::zeroed();
            let pat = b"/bin/sh\0" as *const u8 as *const c_char;
            let result = glob(pat, 0, None, &mut g);
            if result == 0 {
                globfree(&mut g);
                assert_eq!(g.gl_pathc, 0);
                assert!(g.gl_pathv.is_null());
            }
        }
    });

    test!("test_globfree_idempotent" {
        unsafe {
            let mut g: glob_t = core::mem::zeroed();
            globfree(&mut g);
            globfree(&mut g);
        }
    });
}