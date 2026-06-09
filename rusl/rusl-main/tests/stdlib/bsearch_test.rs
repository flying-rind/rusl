/// 模块: bsearch_test
/// `bsearch` 集成测试

use core::ffi::{c_void};
use super::imports::{bsearch};
use test_framework::test;

/// 辅助比较函数：比较两个 i32 值。
unsafe extern "C" fn cmp_i32(a: *const c_void, b: *const c_void) -> i32 {
    let x = *(a as *const i32);
    let y = *(b as *const i32);
    x.cmp(&y) as i32
}

test!("test_find_existing" {
    // 测试 bsearch 在有序数组中查找存在的元素。
    unsafe {
        let arr = [1i32, 3, 5, 7, 9];
        let key = 5i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(!result.is_null());
        assert_eq!(*(result as *const i32), 5);
    }
});

test!("test_not_found" {
    // 测试 bsearch 查找不存在的元素。
    unsafe {
        let arr = [1i32, 3, 5, 7, 9];
        let key = 4i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(result.is_null());
    }
});

test!("test_empty_array" {
    // 测试 bsearch 在空数组中查找（nel=0）。
    unsafe {
        let key = 42i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            core::ptr::null(),
            0,
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(result.is_null());
    }
});

test!("test_single_element_found" {
    // 测试 bsearch 查找单元素数组。
    unsafe {
        let arr = [42i32];
        let key = 42i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(!result.is_null());
        assert_eq!(*(result as *const i32), 42);
    }
});

test!("test_single_element_not_found" {
    // 测试 bsearch 查找单元素数组中的不存在的值。
    unsafe {
        let arr = [42i32];
        let key = 1i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(result.is_null());
    }
});

test!("test_find_first" {
    // 测试 bsearch 查找第一个元素。
    unsafe {
        let arr = [1i32, 2, 3, 4, 5];
        let key = 1i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(!result.is_null());
        assert_eq!(*(result as *const i32), 1);
    }
});

test!("test_find_last" {
    // 测试 bsearch 查找最后一个元素。
    unsafe {
        let arr = [1i32, 2, 3, 4, 5];
        let key = 5i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(!result.is_null());
        assert_eq!(*(result as *const i32), 5);
    }
});

test!("test_find_mid" {
    // 测试 bsearch 查找中位元素。
    unsafe {
        let arr = [1i32, 2, 3, 4, 5];
        let key = 3i32;
        let result = bsearch(
            &key as *const i32 as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_i32),
        );
        assert!(!result.is_null());
        assert_eq!(*(result as *const i32), 3);
    }
});

test!("test_large_width" {
    // 测试 bsearch 处理大宽度类型（如结构体）。
    unsafe {
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        struct Point {
            x: i32,
            y: i32,
        }

        unsafe extern "C" fn cmp_point(a: *const c_void, b: *const c_void) -> i32 {
            let p1 = *(a as *const Point);
            let p2 = *(b as *const Point);
            p1.x.cmp(&p2.x) as i32
        }

        let arr = [
            Point { x: 1, y: 10 },
            Point { x: 3, y: 30 },
            Point { x: 5, y: 50 },
        ];
        let key = Point { x: 3, y: 0 };

        let result = bsearch(
            &key as *const Point as *const c_void,
            arr.as_ptr() as *const c_void,
            arr.len(),
            core::mem::size_of::<Point>(),
            Some(cmp_point),
        );
        assert!(!result.is_null());
        let rv = *(result as *const Point);
        assert_eq!(rv.x, 3);
        assert_eq!(rv.y, 30);
    }
});
