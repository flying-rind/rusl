//! sync 函数集成测试

use test_framework::test;

test!("test_sync_basic" {
    {
        // sync() takes no arguments and returns nothing. Just verify it doesn't crash.
        super::sync();
    }
});

test!("test_sync_idempotent" {
    {
        // Calling sync multiple times should be safe
        super::sync();
        super::sync();
    }
});
