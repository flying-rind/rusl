//! gets 集成测试
//!
//! gets 从 stdin 读取。因为无法可靠控制 stdin, 主要测试边界条件。

use super::imports::gets;
use test_framework::test;

// ---- gets 边界测试 ----

// musl gets 不检查 NULL 缓冲区, 跳过 NULL 测试
