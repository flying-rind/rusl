/// 模块: hsearch_test
/// `hcreate` 集成测试

use core::ffi::{c_char, c_void};
use super::imports::{ACTION, ENTRY, hcreate, hdestroy, hsearch};

use core::ptr;
use rusl_core::test;

extern crate alloc;

test!("test_hcreate_basic" {
    // 测试基本创建哈希表（成功场景）。
    unsafe {
        let ret = hcreate(16);
        assert_ne!(ret, 0);
        hdestroy();
    }
});

test!("test_hcreate_zero" {
    // 测试创建零容量哈希表（边界情况）。
    unsafe {
        let ret = hcreate(0);
        // MINSIZE=8，所以 0 应该成功（会分配 8 个条目的表）
        assert_ne!(ret, 0);
        hdestroy();
    }
});

test!("test_hcreate_large" {
    // 测试创建极大容量哈希表（边界情况）。
    unsafe {
        let ret = hcreate(1024 * 1024);
        // 可能因内存不足返回 0
        if ret != 0 {
            hdestroy();
        }
    }
});

test!("test_hcreate_hdestroy" {
    // 测试创建后销毁的基本流程。
    unsafe {
        hcreate(16);
        hdestroy();
        // 再次创建-销毁应在无先前状态下正常工作
        hcreate(32);
        hdestroy();
    }
});

test!("test_hdestroy_twice" {
    // 测试连续销毁（不应导致 double-free UB，函数内应有保护）。
    unsafe {
        hcreate(8);
        hdestroy();
        // 第二次销毁：若实现正确应安全处理（空检查）
        hdestroy();
    }
});

test!("test_hsearch_find_missing" {
    // 测试 hsearch 查找时返回 null（空表中查找）。
    unsafe {
        hcreate(16);
        let item = ENTRY {
            key: b"missing\0".as_ptr() as *mut c_char,
            data: ptr::null_mut(),
        };
        let result = hsearch(item, ACTION::FIND);
        assert_eq!(result, ptr::null_mut());
        hdestroy();
    }
});

test!("test_hsearch_enter_new" {
    // 测试 hsearch ENTER 插入新条目。
    unsafe {
        hcreate(16);
        let mut data_val: i32 = 42;
        let item = ENTRY {
            key: b"newkey\0".as_ptr() as *mut c_char,
            data: &mut data_val as *mut i32 as *mut c_void,
        };
        let result = hsearch(item, ACTION::ENTER);
        assert!(!result.is_null());
        hdestroy();
    }
});

test!("test_hsearch_enter_existing" {
    // 测试 hsearch ENTER 已存在条目（应返回已存在的指针而非新建）。
    unsafe {
        hcreate(16);
        let key = b"dupkey\0".as_ptr() as *mut c_char;
        let mut data1: i32 = 100;
        let item1 = ENTRY { key, data: &mut data1 as *mut i32 as *mut c_void };
        let r1 = hsearch(item1, ACTION::ENTER);
        assert!(!r1.is_null());

        let mut data2: i32 = 200;
        let item2 = ENTRY { key, data: &mut data2 as *mut i32 as *mut c_void };
        let r2 = hsearch(item2, ACTION::ENTER);
        // 对相同 key 第二次 ENTER 应返回相同指针
        assert_eq!(r1, r2);
        // musl 的 hsearch 在 key 已存在时不更新 data，返回原条目
        assert_eq!((*r2).data, item1.data);

        hdestroy();
    }
});

test!("test_hsearch_find_existing" {
    // 测试 hsearch FIND 查找已存在的条目。
    unsafe {
        hcreate(16);
        let key = b"findme\0".as_ptr() as *mut c_char;
        let mut data: i32 = 77;
        let item = ENTRY { key, data: &mut data as *mut i32 as *mut c_void };
        hsearch(item, ACTION::ENTER);

        let item_find = ENTRY { key, data: ptr::null_mut() };
        let found = hsearch(item_find, ACTION::FIND);
        assert!(!found.is_null());
        assert_eq!((*found).data, item.data);

        hdestroy();
    }
});

test!("test_hsearch_multiple_entries" {
    // 测试多次插入后查找的完整性。
    unsafe {
        hcreate(32);
        let key_strings = [
            alloc::ffi::CString::new("alpha").unwrap(),
            alloc::ffi::CString::new("beta").unwrap(),
            alloc::ffi::CString::new("gamma").unwrap(),
            alloc::ffi::CString::new("delta").unwrap(),
        ];
        for (i, ks) in key_strings.iter().enumerate() {
            let item = ENTRY {
                key: ks.as_ptr() as *mut c_char,
                data: i as *mut c_void,
            };
            hsearch(item, ACTION::ENTER);
        }
        // 验证每个 key 都能被找到
        for (i, ks) in key_strings.iter().enumerate() {
            let item = ENTRY {
                key: ks.as_ptr() as *mut c_char,
                data: core::ptr::null_mut(),
            };
            let found = hsearch(item, ACTION::FIND);
            assert!(!found.is_null());
            assert_eq!((*found).data as usize, i);
        }
        hdestroy();
    }
});

test!("test_hcreate_return_value" {
    // 测试 hcreate 返回 0 的失败场景。
    unsafe {
        let ret = hcreate(usize::MAX);
        // 极大规模应因内存不足返回 0
        assert_eq!(ret, 0);
    }
});
