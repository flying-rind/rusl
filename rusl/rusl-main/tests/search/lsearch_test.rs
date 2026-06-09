/// 模块: lsearch_test
/// `lsearch` 集成测试

use core::ffi::{c_void};
use super::imports::{lsearch, lfind};

use core::ptr;
use test_framework::test;

/// 比较 i32 值的辅助函数。
unsafe extern "C" fn cmp_int(a: *const c_void, b: *const c_void) -> i32 {
    let va = *(a as *const i32);
    let vb = *(b as *const i32);
    if va < vb { -1 } else if va > vb { 1 } else { 0 }
}

test!("test_lfind_finds_existing" {
    // 测试 lfind 找到已存在的元素。
    {
        let arr = [10i32, 20, 30, 40, 50];
        let mut n: usize = 5;
        let key: i32 = 30;
        let result = lfind(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        // 应返回 &arr[2]
        assert_eq!(result, &arr[2] as *const i32 as *mut c_void);
    }
});

test!("test_lfind_not_found" {
    // 测试 lfind 在元素不存在时返回 null。
    {
        let arr = [1i32, 2, 3];
        let mut n: usize = 3;
        let key: i32 = 99;
        let result = lfind(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(result, ptr::null_mut());
    }
});

test!("test_lfind_empty" {
    // 测试 lfind 在空数组中始终返回 null。
    {
        let arr: [i32; 0] = [];
        let mut n: usize = 0;
        let key: i32 = 1;
        let result = lfind(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(result, ptr::null_mut());
        assert_eq!(n, 0);
    }
});

test!("test_lsearch_finds_existing" {
    // 测试 lsearch 找到已存在的元素（不追加）。
    {
        let mut arr = [10i32, 20, 30, 40, 50];
        let mut n: usize = 5;
        let key: i32 = 20;
        let result = lsearch(
            &key as *const i32 as *const c_void,
            arr.as_mut_ptr() as *mut c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        // 应返回 &arr[1]
        assert_eq!(result, &arr[1] as *const i32 as *mut c_void);
        assert_eq!(n, 5);
    }
});

test!("test_lsearch_adds_new" {
    // 测试 lsearch 在未找到时将 key 追加到数组末尾。
    {
        // 预留空间给新元素
        let mut storage = [10i32, 20, 30, 0];
        let mut n: usize = 3;
        let key: i32 = 99;
        let result = lsearch(
            &key as *const i32 as *const c_void,
            storage.as_mut_ptr() as *mut c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(n, 4);
        assert_eq!(storage[3], 99);
        assert_eq!(result, &storage[3] as *const i32 as *mut c_void);
    }
});

test!("test_lsearch_empty" {
    // 测试 lsearch 在空数组中追加。
    {
        let mut storage = [0i32];
        let mut n: usize = 0;
        let key: i32 = 42;
        let result = lsearch(
            &key as *const i32 as *const c_void,
            storage.as_mut_ptr() as *mut c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(n, 1);
        assert_eq!(storage[0], 42);
        assert!(!result.is_null());
    }
});

test!("test_lsearch_then_lfind" {
    // 测试 lsearch 与 lfind 配合使用（先追加再查找）。
    {
        let mut storage = [1i32, 2, 3, 0];
        let mut n: usize = 3;

        // 追加 4
        let key4: i32 = 4;
        lsearch(
            &key4 as *const i32 as *const c_void,
            storage.as_mut_ptr() as *mut c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );

        // 用 lfind 验证 4 存在
        let result = lfind(
            &key4 as *const i32 as *const c_void,
            storage.as_ptr() as *const c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert!(!result.is_null());
    }
});

test!("test_lfind_first_match" {
    // 测试具有相同值的多个元素（返回第一个匹配）。
    {
        let arr = [5i32, 5, 5];
        let mut n: usize = 3;
        let key: i32 = 5;
        let result = lfind(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            &mut n as *mut usize,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        // 应返回第一个元素（arr[0]）
        assert_eq!(result, &arr[0] as *const i32 as *mut c_void);
    }
});

test!("test_lsearch_struct" {
    // 测试宽度不为 1 的元素类型（如结构体）。
    #[repr(C)]
    struct Pair {
        x: i32,
        y: i32,
    }

    unsafe extern "C" fn cmp_pair(a: *const c_void, b: *const c_void) -> i32 {
        let pa = &*(a as *const Pair);
        let pb = &*(b as *const Pair);
        if pa.x < pb.x { -1 } else if pa.x > pb.x { 1 } else { 0 }
    }

    {
        let mut storage = [
            Pair { x: 1, y: 10 },
            Pair { x: 2, y: 20 },
            Pair { x: 0, y: 0 },
        ];
        let mut n: usize = 2;
        let key = Pair { x: 3, y: 30 };

        let result = lsearch(
            &key as *const Pair as *const c_void,
            storage.as_mut_ptr() as *mut c_void,
            &mut n as *mut usize,
            core::mem::size_of::<Pair>(),
            Some(cmp_pair),
        );
        assert_eq!(n, 3);
        assert_eq!(storage[2].x, 3);
        assert_eq!(storage[2].y, 30);
        assert_eq!(result, &storage[2] as *const Pair as *mut c_void);
    }
});
