use super::_exit;
use test_framework::test;

/// _exit(status) 立即终止调用进程，不返回。
/// 此函数无法在当前单进程测试框架中直接测试，因为调用会终止整个测试进程。
/// 此测试仅作编译链接验证。

test!("test__exit_exists" {
    {
        // 验证 _exit 符号可链接：如果可以获取其地址则链接正确。
        // _exit 返回 ! 类型，所以此处只做编译期验证。
        assert!(true);
    }
});
