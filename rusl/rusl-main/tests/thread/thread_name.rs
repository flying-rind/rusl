//! 线程命名 API 集成测试 (GNU 扩展)
//! 测试函数: pthread_setname_np, pthread_getname_np

use super::*;
use test_framework::test;

// ============================================================================
// pthread_setname_np / pthread_getname_np 测试
// ============================================================================

test!("test_setname_getname_self" {
    // 设置和获取当前线程的名称
    {
        let self_id = pthread_self();

        // 设置线程名称
        let name: [c_char; 16] = [
            b't' as c_char, b'e' as c_char, b's' as c_char, b't' as c_char,
            b'_' as c_char, b't' as c_char, b'h' as c_char, b'r' as c_char,
            b'e' as c_char, b'a' as c_char, b'd' as c_char,
            0, 0, 0, 0, 0,
        ];
        let ret = pthread_setname_np(self_id, name.as_ptr());
        assert_eq!(ret, 0, "setname_np 应返回 0");

        // 获取线程名称
        let mut buf: [c_char; 32] = [0; 32];
        let ret = pthread_getname_np(self_id, buf.as_mut_ptr(), buf.len());
        assert_eq!(ret, 0, "getname_np 应返回 0");

        // 验证名称以 "test_th" 开头
        let mut match_prefix = true;
        let expected: [u8; 7] = [b't', b'e', b's', b't', b'_', b't', b'h'];
        for i in 0..7 {
            if buf[i] as u8 != expected[i] {
                match_prefix = false;
                break;
            }
        }
        assert!(match_prefix, "线程名称应以 'test_th' 开头");
    }
});

test!("test_setname_np_long_name" {
    // 设置超长名称, 可能返回 ERANGE 或截断
    {
        let self_id = pthread_self();

        // 创建一个 30 字符的名称
        let long_name: [c_char; 31] = [
            b'A' as c_char, b'B' as c_char, b'C' as c_char, b'D' as c_char, b'E' as c_char,
            b'F' as c_char, b'G' as c_char, b'H' as c_char, b'I' as c_char, b'J' as c_char,
            b'K' as c_char, b'L' as c_char, b'M' as c_char, b'N' as c_char, b'O' as c_char,
            b'P' as c_char, b'Q' as c_char, b'R' as c_char, b'S' as c_char, b'T' as c_char,
            b'U' as c_char, b'V' as c_char, b'W' as c_char, b'X' as c_char, b'Y' as c_char,
            b'Z' as c_char, b'1' as c_char, b'2' as c_char, b'3' as c_char, b'4' as c_char,
            0,
        ];

        let ret = pthread_setname_np(self_id, long_name.as_ptr());
        // musl 限制线程名为 15 字符，超长返回 ERANGE(34)
        assert!(
            ret == 0 || ret == 34, // 34 = ERANGE
            "setname_np 应返回 0 或 ERANGE, got {}",
            ret
        );
    }
});

test!("test_getname_np_small_buffer" {
    // 使用小缓冲区获取名称, musl 对小缓冲区返回 ERANGE(34)
    {
        // 使用只有 4 字节的缓冲区
        let mut buf: [c_char; 4] = [0; 4];
        let ret = pthread_getname_np(pthread_self(), buf.as_mut_ptr(), buf.len());
        // musl 缓冲区不足时返回 ERANGE(34)
        assert!(
            ret == 0 || ret == 34, // 34 = ERANGE
            "getname_np 应返回 0 或 ERANGE, got {}",
            ret
        );
    }
});
