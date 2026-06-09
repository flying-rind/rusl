#![allow(useless_ptr_null_checks)]
//! ctype 域集成测试
//!
//! 测试 isalpha/isdigit/isspace 等字符分类 C ABI 接口。

use test_framework::test;
pub use core::ffi::c_int;
pub use core::ffi::c_void;
pub use core::ffi::c_char;

mod ctype_get_mb_cur_max_test;
mod ctype_integration_test;
mod isalpha_test;
mod isascii_test;
mod isblank_test;
mod iscntrl_test;
mod isdigit_test;
mod isgraph_test;
mod islower_test;
mod isprint_test;
mod ispunct_test;
mod isspace_test;
mod isupper_test;
mod iswalnum_test;
mod iswalpha_test;
mod iswblank_test;
mod iswcntrl_test;
mod iswctype_test;
mod iswdigit_test;
mod iswgraph_test;
mod iswlower_test;
mod iswprint_test;
mod iswpunct_test;
mod iswspace_test;
mod iswupper_test;
mod iswxdigit_test;
mod isxdigit_test;
mod toascii_test;
mod tolower_test;
mod toupper_test;
mod towctrans_test;
mod wcswidth_test;
mod wctrans_test;
mod wcwidth_test;

// 根据 rusl feature 选择导入源
#[cfg(feature = "rusl")]
mod imports {
    pub use rusl_ctype::*;
    pub use rusl::api::types::*;
}
#[cfg(not(feature = "rusl"))]
mod imports {
    pub use rusl::api::ctype::*;
    pub use rusl::api::types::*;
}
pub use imports::*;