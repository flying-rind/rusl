//! tsearch — AVL 自平衡二叉树搜索/插入。
//! 对 C ABI 导出符号：`tsearch`。
//!
//! 包含 AVL 旋转（`rot`）和再平衡（`__tsearch_balance`）内部函数，
//! 这些函数也是 tdelete 模块所依赖的公共内部 API。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use core::ptr;
use super::types::Node;

/// AVL 树高度安全上界（< 1.44*log2(nodes+2)-0.3）。
/// `sizeof(void*)*8*3/2`：64 位下为 96，32 位下为 48。
const MAXH: usize = core::mem::size_of::<*mut c_void>() * 8 * 3 / 2;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

/// 获取节点高度（null 节点高度为 0）。
#[inline]
unsafe fn height(n: *mut c_void) -> i32 {
    if n.is_null() { 0 } else { (*(n as *mut Node)).h }
}

/// AVL 旋转操作。
///
/// 对子树进行单旋转或双旋转，使 `dir` 侧（更深的一侧）恢复平衡。
/// - `p`：指向子树链接的指针（`*p` 是子树的根）。
/// - `x`：当前子树的根节点。
/// - `dir`：`false` 代表左侧更深，`true` 代表右侧更深。
///
/// 返回子树高度变化量（高度差）。
unsafe fn rot(p: *mut *mut c_void, x: *mut Node, dir: bool) -> i32 {
    let dir_idx = dir as usize;
    let not_dir = (!dir) as usize;

    let y = (*x).a[dir_idx] as *mut Node;
    let z = (*y).a[not_dir] as *mut Node;
    let hx = (*x).h;
    let hz = height(z as *mut c_void);

    if hz > height((*y).a[dir_idx]) {
        // 双旋转：
        //        x                      z
        //       / \                    / \
        //      A   y        -->       x   y
        //         / \                / \ / \
        //        z   D              A  B C  D
        //       / \
        //      B   C
        (*x).a[dir_idx] = (*z).a[not_dir];
        (*y).a[not_dir] = (*z).a[dir_idx];
        (*z).a[not_dir] = x as *mut c_void;
        (*z).a[dir_idx] = y as *mut c_void;
        (*x).h = hz;
        (*y).h = hz;
        (*z).h = hz + 1;
        *p = z as *mut c_void;
        (*z).h - hx
    } else {
        // 单旋转：
        //        x                    y
        //       / \                  / \
        //      A   y     -->        x   D
        //         / \              / \
        //        z   D            A   z
        (*x).a[dir_idx] = z as *mut c_void;
        (*y).a[not_dir] = x as *mut c_void;
        (*x).h = hz + 1;
        (*y).h = hz + 2;
        *p = y as *mut c_void;
        (*y).h - hx
    }
}

/// AVL 再平衡。
///
/// 检查 `*p` 节点的平衡因子，若失衡则旋转，若平衡则更新高度。
/// - 返回 0 表示高度未变化，非 0 表示高度变化（需要继续向上平衡）。
pub(crate) unsafe fn __tsearch_balance(p: *mut *mut c_void) -> i32 {
    let n = *p as *mut Node;
    let h0 = height((*n).a[0]);
    let h1 = height((*n).a[1]);

    // 检查是否平衡：|h0 - h1| <= 1
    // （等效于 C 源码中的 `h0 - h1 + 1u < 3u` 检查）
    if (h0 - h1).abs() <= 1 {
        let old = (*n).h;
        (*n).h = if h0 < h1 { h1 + 1 } else { h0 + 1 };
        return (*n).h - old;
    }

    // 不平衡：需要旋转
    // dir = true (1) 表示右侧更深，false (0) 表示左侧更深
    rot(p, n, h0 < h1)
}

/// 在二叉树中搜索 `key`，若不存在则插入新节点（内部实现）。
///
/// 使用 AVL 自平衡算法。树节点存储的是 `key` 指针值（不复制数据）。
///
/// - `*rootp` 为 null 表示空树。
/// - 若 `key` 已存在：返回匹配节点的指针。
/// - 若 `key` 不存在：分配新节点，插入并重平衡，返回新节点指针。
/// - 内存不足时返回 null。
///
/// # Safety
///
/// 调用者必须确保：
/// - `rootp` 为有效非空指针。
/// - `compar` 为有效的比较函数，接收两个 `*const c_void` 返回负数/零/正数。
/// - `key` 指向的数据在树的生命周期内保持有效和不变（节点仅存储指针）。
pub(crate) unsafe fn tsearch_impl(
    key: *const c_void,
    rootp: *mut *mut c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void {
    if rootp.is_null() {
        return ptr::null_mut();
    }

    let cmp = match compar {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    // 路径栈：记录从根到插入点的所有指针地址
    let mut a: [*mut *mut c_void; MAXH] = [ptr::null_mut(); MAXH];
    let mut n: *mut Node = (*rootp) as *mut Node;
    let mut i: usize = 0;

    a[i] = rootp;
    i += 1;

    // 沿树查找，记录路径
    loop {
        if n.is_null() {
            break;
        }
        let c = cmp(key, (*n).key);
        if c == 0 {
            // 已存在，直接返回
            return n as *mut c_void;
        }
        let dir = (c > 0) as usize;
        a[i] = &mut (*n).a[dir] as *mut *mut c_void;
        i += 1;
        n = (*n).a[dir] as *mut Node;
    }

    // 创建新节点
    let r = malloc(core::mem::size_of::<Node>()) as *mut Node;
    if r.is_null() {
        return ptr::null_mut();
    }
    (*r).key = key;
    (*r).a = [ptr::null_mut(), ptr::null_mut()];
    (*r).h = 1;

    // 插入新节点到树中
    i -= 1;
    *a[i] = r as *mut c_void;

    // 从新节点的父节点开始向上再平衡，直到根节点
    // C 源码：while (i && __tsearch_balance(a[--i]));
    // 等价于：先检查 i 非零，然后递减 i，再平衡
    loop {
        if i == 0 {
            break;
        }
        i -= 1;
        if __tsearch_balance(a[i]) == 0 {
            break;
        }
    }

    r as *mut c_void
}

/// 在二叉树中搜索 `key`，若不存在则插入新节点（C ABI 导出符号）。
///
/// 内部委托给 `tsearch_impl`。测试模式下不生成此符号，
/// 测试通过 `tsearch_impl` 直接调用，避免与 libc 的 `tsearch` 符号冲突。
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn tsearch(
    key: *const c_void,
    rootp: *mut *mut c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void {
    tsearch_impl(key, rootp, compar)
}

#[cfg(test)]
mod tests {
    use rusl_core::test;

        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use alloc::string::String;
    use super::*;
    use core::ptr;

    /// 比较 i32 值的辅助函数。
    unsafe extern "C" fn cmp_int(a: *const c_void, b: *const c_void) -> i32 {
        let va = *(a as *const i32);
        let vb = *(b as *const i32);
        if va < vb { -1 } else if va > vb { 1 } else { 0 }
    }

    test!("test_tsearch_insert_empty" {
        // 测试在空树中插入第一个元素。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let key: i32 = 42;
            let result = tsearch_impl(
                &key as *const i32 as *const c_void,
                &mut root as *mut *mut c_void,
                Some(cmp_int),
            );
            assert!(!result.is_null());
            assert!(!root.is_null());
        }
    });

    test!("test_tsearch_duplicate" {
        // 测试插入已存在的 key（应返回已有节点，不重复插入）。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let key: i32 = 10;
            let r1 = tsearch_impl(
                &key as *const i32 as *const c_void,
                &mut root as *mut *mut c_void,
                Some(cmp_int),
            );
            let r2 = tsearch_impl(
                &key as *const i32 as *const c_void,
                &mut root as *mut *mut c_void,
                Some(cmp_int),
            );
            // 第二次插入应返回与第一次相同的指针
            assert_eq!(r1, r2);
        }
    });

    test!("test_tsearch_multiple" {
        // 测试插入多个元素并验证树结构。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let keys = [5i32, 3, 7, 2, 4, 6, 8];
            let mut results: [*mut c_void; 7] = [ptr::null_mut(); 7];

            for (i, k) in keys.iter().enumerate() {
                results[i] = tsearch_impl(
                    k as *const i32 as *const c_void,
                    &mut root as *mut *mut c_void,
                    Some(cmp_int),
                );
                assert!(!results[i].is_null());
            }

            // 所有 key 都可被找到
            for (i, k) in keys.iter().enumerate() {
                let found = tsearch_impl(
                    k as *const i32 as *const c_void,
                    &mut root as *mut *mut c_void,
                    Some(cmp_int),
                );
                assert_eq!(found, results[i]);
            }
        }
    });

    test!("test_tsearch_null_rootp" {
        // 测试 null rootp（应返回 null）。
        unsafe {
            let key: i32 = 1;
            let result = tsearch_impl(
                &key as *const i32 as *const c_void,
                ptr::null_mut(),
                Some(cmp_int),
            );
            assert!(result.is_null());
        }
    });

    test!("test_tsearch_null_root" {
        // 测试 rootp 指向的根指针为 null（空树插入）。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let key: i32 = 100;
            let result = tsearch_impl(
                &key as *const i32 as *const c_void,
                &mut root as *mut *mut c_void,
                Some(cmp_int),
            );
            assert!(!result.is_null());
            assert!(!root.is_null());
        }
    });

    test!("test_tsearch_null_compar" {
        // 测试使用 null compar（返回 null 而非崩溃）。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let key: i32 = 1;
            let result = tsearch_impl(
                &key as *const i32 as *const c_void,
                &mut root as *mut *mut c_void,
                None,
            );
            assert!(result.is_null());
        }
    });

    test!("test_tsearch_many_elements" {
        // 测试大量元素插入，验证不发生栈溢出或逻辑错误。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let count = 100;
            let keys: Vec<i32> = (0..count).collect();
            for k in &keys {
                let result = tsearch_impl(
                    k as *const i32 as *const c_void,
                    &mut root as *mut *mut c_void,
                    Some(cmp_int),
                );
                assert!(!result.is_null());
            }
        }
    });

    test!("test_tsearch_key_integrity" {
        // 测试 tsearch 返回的指针指向的 key 值与插入时一致。
        unsafe {
            let mut root: *mut c_void = ptr::null_mut();
            let key: i32 = 77;
            let result = tsearch_impl(
                &key as *const i32 as *const c_void,
                &mut root as *mut *mut c_void,
                Some(cmp_int),
            );
            if !result.is_null() {
                let node = &*(result as *const Node);
                assert_eq!(node.key as *const i32, &key as *const i32);
            }
        }
    });
}