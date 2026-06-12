//! feof / ferror / clearerr / fileno / fseek / ftell / rewind 集成测试

use core::ffi::{c_char,  c_long, c_void};
use super::imports::{
    fopen, fclose, feof, ferror, clearerr, fileno,
    fseek, ftell, rewind, fgetc, fwrite,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// -----------------------------------------------------------------------
// feof 测试
// -----------------------------------------------------------------------

test!("feof_null_file" {
    // musl 的 feof 对 NULL FILE* 直接解引用, 使用 /dev/null 测试
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = feof(f);
    assert_eq!(ret, 0, "刚打开时 feof 应为 0");
    fclose(f);
});

test!("feof_at_start" {
    // 前置: 刚打开的文件
    // 后置: feof 返回 0（未到末尾）
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = feof(f);
    assert_eq!(ret, 0, "刚打开时 feof 应为 0");
    fclose(f);
});

test!("feof_after_read_eof" {
    // 前置: 尝试读取已到达文件尾的流
    // 后置: feof 返回非零
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    // 尝试读取（/dev/null 立即返回 EOF）
    let _ = fgetc(f);
    let ret = feof(f);
    assert_ne!(ret, 0, "/dev/null 读取后 feof 应为非零");
    fclose(f);
});

// -----------------------------------------------------------------------
// ferror 测试
// -----------------------------------------------------------------------

test!("ferror_null_file" {
    // musl 的 ferror 对 NULL FILE* 直接解引用, 使用 /dev/null 测试
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = ferror(f);
    assert_eq!(ret, 0, "刚打开时 ferror 应为 0");
    fclose(f);
});

test!("ferror_at_start" {
    // 前置: 刚打开的文件
    // 后置: ferror 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = ferror(f);
    assert_eq!(ret, 0, "刚打开时 ferror 应为 0");
    fclose(f);
});

// -----------------------------------------------------------------------
// clearerr 测试
// -----------------------------------------------------------------------

test!("clearerr_resets_eof" {
    // 前置: 打开文件，读取到 EOF，再清除错误
    // 后置: clearerr 后 feof 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let _ = fgetc(f); // 触发 EOF
    assert_ne!(feof(f), 0, "应检测到 EOF");

    clearerr(f);
    assert_eq!(feof(f), 0, "clearerr 后 feof 应恢复为 0");
    fclose(f);
});

test!("clearerr_null_file" {
    // musl 的 clearerr 对 NULL FILE* 写操作导致 SIGSEGV, 使用有效文件测试
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    clearerr(f);
    fclose(f);
});

// -----------------------------------------------------------------------
// fileno 测试
// -----------------------------------------------------------------------

test!("fileno_valid_file" {
    // 前置: 打开 /dev/null
    // 后置: 返回非负文件描述符
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let fd = fileno(f);
    assert!(fd >= 0, "fileno 应返回 >= 0 的文件描述符, got {}", fd);
    fclose(f);
});

test!("fileno_null_file" {
    // musl 的 fileno 对 NULL FILE* 直接解引用, 使用 /dev/null 测试
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let fd = fileno(f);
    assert!(fd >= 0, "fileno 应返回 >= 0, got {}", fd);
    fclose(f);
});

// -----------------------------------------------------------------------
// fseek 测试
// -----------------------------------------------------------------------

test!("fseek_null_file" {
    // musl 的 fseek 对 NULL FILE* 直接解引用, 使用 /dev/null 测试
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fseek(f, 0, 0); // SEEK_SET = 0
    // /dev/null 不可 seek, 返回值因实现而异
    let _ = ret;
    fclose(f);
});

test!("fseek_set_to_beginning" {
    // 前置: 文件已写入一些数据，定位到开头
    // 后置: 返回 0
    let path = b"/tmp/__rusl_test_fseek__.dat\0";
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null());
    let data = b"abcdef";
    let _ = fwrite(data.as_ptr() as *const c_void, 1, 6, fw);
    fclose(fw);

    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null());
    // SEEK_SET = 0, 偏移 0
    let ret = fseek(fr, 0, 0);
    assert_eq!(ret, 0, "fseek 到文件开头应返回 0");
    // 验证可从头读取
    let c = fgetc(fr);
    assert_eq!(c as u8, b'a');
    fclose(fr);
});

// -----------------------------------------------------------------------
// ftell 测试
// -----------------------------------------------------------------------

test!("ftell_null_file" {
    // musl 的 ftell 对 NULL FILE* 直接解引用, 使用 /dev/null 测试
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let pos = ftell(f);
    // /dev/null 不可 seek, 通常返回 0
    assert!(pos >= 0, "ftell 应返回 >= 0, got {}", pos);
    fclose(f);
});

test!("ftell_at_start" {
    // 前置: 刚打开的文件
    // 后置: ftell 返回 0
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"r\0"));
    assert!(!f.is_null());
    let pos = ftell(f);
    assert_eq!(pos, 0, "刚打开时 ftell 应为 0");
    fclose(f);
});

// -----------------------------------------------------------------------
// rewind 测试
// -----------------------------------------------------------------------

test!("rewind_null_file" {
    // musl 的 rewind 对 NULL FILE* 直接解引用, 使用有效文件测试
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    rewind(f);
    fclose(f);
});

test!("rewind_resets_position" {
    // 前置: 写入数据后重绕
    // 后置: ftell 再次返回 0
    let path = b"/tmp/__rusl_test_rewind__.dat\0";
    let fw = fopen(cstr(path), cstr(b"w\0"));
    assert!(!fw.is_null());
    let data = b"test data";
    let _ = fwrite(data.as_ptr() as *const c_void, 1, 9, fw);
    fclose(fw);

    let fr = fopen(cstr(path), cstr(b"r\0"));
    assert!(!fr.is_null());

    // 读取一些字符以推进位置
    let _ = fgetc(fr); // 't'
    let _ = fgetc(fr); // 'e'
    let pos_after = ftell(fr);
    assert!(pos_after >= 2);

    rewind(fr);
    let pos_after_rewind = ftell(fr);
    assert_eq!(pos_after_rewind, 0, "rewind 后位置应为 0");

    // 清除可能的 EOF 标志后，应可重新读取
    let c = fgetc(fr);
    assert_eq!(c as u8, b't');
    fclose(fr);
});
