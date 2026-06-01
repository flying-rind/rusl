// C standard type aliases used throughout the library.
// These match the definitions in musl's include/alltypes.h.in

pub type size_t    = usize;
pub type ssize_t   = isize;
pub type off_t     = i64;
pub type mode_t    = u32;
pub type pid_t     = i32;
pub type uid_t     = u32;
pub type gid_t     = u32;
pub type nlink_t   = u64;      // _Reg type; on x86_64 _Reg = unsigned long
pub type ino_t     = u64;
pub type dev_t     = u64;
pub type blksize_t = i32;      // long on x86_64
pub type blkcnt_t  = i64;
pub type time_t    = i64;
pub type suseconds_t = i64;
pub type clock_t   = i64;
pub type clockid_t = i32;
pub type wchar_t   = i32;

// int types
pub type int8_t    = i8;
pub type int16_t   = i16;
pub type int32_t   = i32;
pub type int64_t   = i64;
pub type uint8_t   = u8;
pub type uint16_t  = u16;
pub type uint32_t  = u32;
pub type uint64_t  = u64;

pub type intmax_t  = i64;
pub type uintmax_t = u64;
pub type intptr_t  = isize;
pub type uintptr_t = usize;

// 宽字符与 locale 类型，供 ctype 及 wchar 模块使用
pub type wint_t    = core::ffi::c_uint;   // 宽字符类型 (unsigned int)，可存储 WEOF
pub type wctype_t  = core::ffi::c_ulong;  // 字符分类标识符类型 (unsigned long)
pub type wctrans_t = core::ffi::c_ulong;  // 大小写变换描述符类型 (unsigned long)
pub type locale_t  = *mut core::ffi::c_void; // locale 句柄类型

// 宽字符与 locale 常量
/// WEOF: 对应 C 的 `#define WEOF 0xffffffffU`（来自 `<wctype.h>`）。
/// 宽字符 EOF 标记值。
pub const WEOF: wint_t = 0xffffffff_u32;