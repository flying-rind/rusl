/// 模块: qsort_test
/// `__qsort_r` 集成测试

use core::ffi::{c_void};
use super::imports::{qsort_r, qsort};
use test_framework::test;

// 辅助比较函数
unsafe extern "C" fn cmp_int(a: *const c_void, b: *const c_void) -> i32 {
    let x = *(a as *const i32);
    let y = *(b as *const i32);
    x.cmp(&y) as i32
}

unsafe extern "C" fn cmp_int_r(a: *const c_void, b: *const c_void, _arg: *mut c_void) -> i32 {
    let x = *(a as *const i32);
    let y = *(b as *const i32);
    x.cmp(&y) as i32
}

unsafe extern "C" fn cmp_int_rev(a: *const c_void, b: *const c_void) -> i32 {
    let x = *(a as *const i32);
    let y = *(b as *const i32);
    y.cmp(&x) as i32
}

// 使用 qsort 对数组进行排序 (替代 .sort() 方法, 兼容 no_std)
unsafe fn sort_i32_array(arr: &mut [i32]) {
    let len = arr.len();
    qsort(
        arr.as_mut_ptr() as *mut c_void,
        len,
        core::mem::size_of::<i32>(),
        Some(cmp_int),
    );
}

// ---- qsort 测试 ----

test!("test_qsort_random" {
    // 测试 qsort 排序随机排列的数组。
    unsafe {
        let mut arr = [3i32, 1, 4, 1, 5, 9, 2, 6, 5, 3];
        let mut expected = arr;
        sort_i32_array(&mut expected);
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(arr, expected);
    }
});

test!("test_qsort_already_sorted" {
    // 测试 qsort 已排序数组（最佳情况）。
    unsafe {
        let mut arr = [1i32, 2, 3, 4, 5, 6, 7, 8, 9];
        let expected = arr;
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(arr, expected);
    }
});

test!("test_qsort_reverse_sorted" {
    // 测试 qsort 逆序数组（最坏情况之一）。
    unsafe {
        let mut arr = [9i32, 8, 7, 6, 5, 4, 3, 2, 1];
        let mut expected = arr;
        sort_i32_array(&mut expected);
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(arr, expected);
    }
});

test!("test_qsort_empty" {
    // 测试 qsort 空数组（nel=0）。
    unsafe {
        let mut arr: [i32; 0] = [];
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            0,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
    }
});

test!("test_qsort_single" {
    // 测试 qsort 单元素数组。
    unsafe {
        let mut arr = [42i32];
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            1,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(arr[0], 42);
    }
});

test!("test_qsort_two" {
    // 测试 qsort 两元素数组。
    unsafe {
        let mut arr = [2i32, 1];
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            2,
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(arr, [1, 2]);
    }
});

test!("test_qsort_all_equal" {
    // 测试 qsort 所有元素相同。
    unsafe {
        let mut arr = [5i32, 5, 5, 5, 5];
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(arr, [5, 5, 5, 5, 5]);
    }
});

test!("test_qsort_negative_numbers" {
    // 测试 qsort 负数值。
    unsafe {
        let mut arr = [-3i32, 5, -1, 0, 2, -8];
        let mut expected = arr;
        sort_i32_array(&mut expected);
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int),
        );
        assert_eq!(arr, expected);
    }
});

test!("test_qsort_reverse_cmp" {
    // 测试 qsort 使用逆序比较函数。
    unsafe {
        let mut arr = [1i32, 2, 3, 4, 5];
        let expected = [5, 4, 3, 2, 1];
        qsort(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int_rev),
        );
        assert_eq!(arr, expected);
    }
});

test!("test_qsort_struct" {
    // 测试 qsort 排序大型结构体。
    unsafe {
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct Point {
            x: i32,
            y: i32,
        }

        unsafe extern "C" fn cmp_point(a: *const c_void, b: *const c_void) -> i32 {
            let p1 = *(a as *const Point);
            let p2 = *(b as *const Point);
            p1.x.cmp(&p2.x) as i32
        }

        let mut arr = [
            Point { x: 5, y: 50 },
            Point { x: 3, y: 30 },
            Point { x: 1, y: 10 },
            Point { x: 4, y: 40 },
            Point { x: 2, y: 20 },
        ];

        qsort(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<Point>(),
            Some(cmp_point),
        );

        for i in 1..arr.len() {
            assert!(arr[i - 1].x <= arr[i].x, "排序失败: {:?}", arr);
        }
    }
});

// ---- qsort_r 测试 ----

test!("test_qsort_r_basic" {
    // 测试 qsort_r 基本功能。
    unsafe {
        let mut arr = [3i32, 1, 4, 1, 5, 9];
        let mut expected = arr;
        sort_i32_array(&mut expected);
        let arg: i32 = 0;
        qsort_r(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int_r),
            &arg as *const i32 as *mut c_void,
        );
        assert_eq!(arr, expected);
    }
});

// ---- __qsort_r 测试 ----

test!("test___qsort_r_basic" {
    // 测试 __qsort_r 基本功能。
    unsafe {
        let mut arr = [9i32, 8, 7, 6, 5];
        let mut expected = arr;
        sort_i32_array(&mut expected);
        let arg: i32 = 0;
        qsort_r(
            arr.as_mut_ptr() as *mut c_void,
            arr.len(),
            core::mem::size_of::<i32>(),
            Some(cmp_int_r),
            &arg as *const i32 as *mut c_void,
        );
        assert_eq!(arr, expected);
    }
});

test!("test___qsort_r_single" {
    // 测试 __qsort_r 单元素直接返回。
    unsafe {
        let mut arr = [42i32];
        qsort_r(
            arr.as_mut_ptr() as *mut c_void,
            1,
            core::mem::size_of::<i32>(),
            Some(cmp_int_r),
            core::ptr::null_mut(),
        );
        assert_eq!(arr[0], 42);
    }
});

test!("test___qsort_r_empty" {
    // 测试 __qsort_r 空数组直接返回。
    unsafe {
        let mut arr: [i32; 0] = [];
        qsort_r(
            arr.as_mut_ptr() as *mut c_void,
            0,
            core::mem::size_of::<i32>(),
            Some(cmp_int_r),
            core::ptr::null_mut(),
        );
    }
});