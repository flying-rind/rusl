/// 模块: random_test
/// `random` 集成测试

use super::*;
use rusl_core::test;

// ─── 测试辅助函数 ────────────────────────────────────────────

unsafe fn reset_global_state() {
    // 重置为 musl 默认状态: srandom(1) 然后 initstate with default
    srandom(1);
}

// ─── random / srandom ───────────────────────────────────────

test!("test_random_range" {
    // random 返回值在 [0, 2^31-1] 范围内。
    unsafe {
        reset_global_state();
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
    }
});

test!("test_srandom_after_seeding" {
    // srandom 后 random 产生非负值。
    unsafe {
        reset_global_state();
        srandom(42);
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
    }
});

test!("test_srandom_reproducibility" {
    // 相同种子产生相同序列（可复现性）。
    unsafe {
        reset_global_state();
        srandom(12345);
        let a = random();
        srandom(12345);
        let b = random();
        assert_eq!(a, b);
    }
});

test!("test_srandom_different_seeds" {
    // 不同种子通常产生不同的初始值。
    unsafe {
        reset_global_state();
        srandom(1);
        let _a = random();
        srandom(2);
        let _b = random();
        // 不 panic 即可
    }
});

test!("test_random_multiple_calls" {
    // random 连续多次调用均返回有效范围。
    unsafe {
        reset_global_state();
        srandom(99);
        for _ in 0..20 {
            let val = random();
            assert!(val >= 0 && val <= 0x7fffffff);
        }
    }
});

test!("test_srandom_zero" {
    // srandom(0) 边界情况。
    unsafe {
        reset_global_state();
        srandom(0);
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
    }
});

// ─── initstate ──────────────────────────────────────────────

test!("test_initstate_valid_size" {
    // initstate 对有效大小返回非空指针（旧状态）。
    unsafe {
        reset_global_state();
        srandom(42); // 确保全局状态已初始化
        let mut state = [0u8; 32];
        let old = initstate(123, state.as_mut_ptr(), 32);
        assert!(!old.is_null(), "initstate 对于 size>=8 应返回非空指针");
        setstate(old); // 恢复旧状态，避免 STATE_X 指向已释放的栈内存
    }
});

test!("test_initstate_size_too_small" {
    // initstate 对 n < 8 返回 null 指针。
    unsafe {
        reset_global_state();
        let mut state = [0u8; 7];
        let result = initstate(42, state.as_mut_ptr(), 7);
        assert!(result.is_null(), "initstate 对于 n<8 应返回 null");
    }
});

test!("test_initstate_size_zero" {
    // initstate 对 n = 0 返回 null。
    unsafe {
        reset_global_state();
        let result = initstate(42, core::ptr::null_mut(), 0);
        assert!(result.is_null(), "initstate 对于 n=0 应返回 null");
    }
});

test!("test_initstate_min_size" {
    // initstate 对 n = 8（最小有效值）返回非空。
    unsafe {
        reset_global_state();
        srandom(42);
        let mut state = [0u8; 8];
        let old = initstate(777, state.as_mut_ptr(), 8);
        assert!(!old.is_null(), "initstate 对于 n=8 应返回非空");
        setstate(old); // 恢复旧状态
    }
});

test!("test_initstate_large_size" {
    // initstate 对大状态表 (n=256) 返回非空。
    unsafe {
        reset_global_state();
        srandom(42);
        let mut state = [0u8; 256];
        let old = initstate(9999, state.as_mut_ptr(), 256);
        assert!(!old.is_null(), "initstate 对于 n=256 应返回非空");
        setstate(old); // 恢复旧状态
    }
});

// ─── setstate ───────────────────────────────────────────────

test!("test_setstate_valid" {
    // setstate 返回非空指针（旧状态）。
    unsafe {
        reset_global_state();
        srandom(42);
        let mut state = [0u8; 32];
        let old1 = initstate(123, state.as_mut_ptr(), 32);

        // 切换到另一个状态
        let mut state2 = [0u8; 32];
        let _old2 = initstate(456, state2.as_mut_ptr(), 32);

        // setstate 切换回 state
        let prev = setstate(state.as_mut_ptr());
        assert!(!prev.is_null(), "setstate 应返回非空指针");

        // 恢复到最初的默认状态
        setstate(old1);
    }
});

// ─── 集成测试 ──────────────────────────────────────────────

test!("test_random_full_workflow" {
    // initstate + srandom + random 完整工作流。
    unsafe {
        reset_global_state();
        // 初始化状态表
        let mut state = [0u8; 64];
        let old = initstate(42, state.as_mut_ptr(), 64);
        assert!(!old.is_null());

        // 生成随机数
        let v1 = random();
        assert!(v1 >= 0 && v1 <= 0x7fffffff);

        // 切换回旧状态
        let _prev = setstate(old);
        let v2 = random();
        assert!(v2 >= 0 && v2 <= 0x7fffffff);
    }
});

test!("test_random_state_switch_reproducible" {
    // 多次调用 initstate/setstate 保持可复现性。
    unsafe {
        reset_global_state();
        // 使用状态表 A
        let mut state_a = [0u8; 32];
        let old_default = initstate(100, state_a.as_mut_ptr(), 32);
        let seq_a1 = random();
        let seq_a2 = random();

        // 使用状态表 B
        let mut state_b = [0u8; 64];
        let _old_b = initstate(200, state_b.as_mut_ptr(), 64);
        let _seq_b1 = random();

        // 重新用相同种子初始化 A，应产生相同序列
        initstate(100, state_a.as_mut_ptr(), 32);
        let seq_a1_new = random();
        let seq_a2_new = random();
        assert_eq!(seq_a1, seq_a1_new, "状态表 A 序列应可复现");
        assert_eq!(seq_a2, seq_a2_new, "状态表 A 序列应可复现");

        // 恢复默认状态
        setstate(old_default);
    }
});

test!("test_initstate_size_8" {
    // 不同大小状态表的行为（验证不 panic）。
    unsafe {
        reset_global_state();
        let mut state = [0u8; 8];
        srandom(42);
        let old = initstate(42, state.as_mut_ptr(), 8);
        assert!(!old.is_null(), "size=8 应返回非空");
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
        setstate(old); // 恢复旧状态
    }
});

test!("test_initstate_size_32" {
    // initstate 大小 = 32。
    unsafe {
        reset_global_state();
        let mut state = [0u8; 32];
        srandom(42);
        let old = initstate(42, state.as_mut_ptr(), 32);
        assert!(!old.is_null(), "size=32 应返回非空");
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
        setstate(old);
    }
});

test!("test_initstate_size_64" {
    // initstate 大小 = 64。
    unsafe {
        reset_global_state();
        let mut state = [0u8; 64];
        srandom(42);
        let old = initstate(42, state.as_mut_ptr(), 64);
        assert!(!old.is_null(), "size=64 应返回非空");
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
        setstate(old);
    }
});

test!("test_initstate_size_128" {
    // initstate 大小 = 128。
    unsafe {
        reset_global_state();
        let mut state = [0u8; 128];
        srandom(42);
        let old = initstate(42, state.as_mut_ptr(), 128);
        assert!(!old.is_null(), "size=128 应返回非空");
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
        setstate(old);
    }
});

test!("test_initstate_size_256" {
    // initstate 大小 = 256。
    unsafe {
        reset_global_state();
        let mut state = [0u8; 256];
        srandom(42);
        let old = initstate(42, state.as_mut_ptr(), 256);
        assert!(!old.is_null(), "size=256 应返回非空");
        let val = random();
        assert!(val >= 0 && val <= 0x7fffffff);
        setstate(old);
    }
});
