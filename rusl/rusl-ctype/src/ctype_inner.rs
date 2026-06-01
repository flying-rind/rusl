//! ctype —— 字符分类函数。
//! 对应 musl src/ctype/
//!
//! 包含字节字符分类 (is*) 和宽字符分类 (isw*) 两组函数。
//! 所有公共接口均为 `extern "C"` ABI, 与 POSIX/C 标准保持兼容。

#![allow(dead_code, unused_imports)]

// 底层 locale 辅助函数
mod __ctype_b_loc;
mod __ctype_get_mb_cur_max;
mod __ctype_tolower_loc;
mod __ctype_toupper_loc;

// 字节字符分类
mod isalpha;
mod isascii;
mod isblank;
mod iscntrl;
mod isdigit;
mod isgraph;
mod islower;
mod isprint;
mod ispunct;
mod isspace;
mod isupper;
mod isxdigit;
mod toascii;

// 大小写转换 (字节)
mod tolower;
mod toupper;

// 大小写转换 (宽字符)
mod towctrans;

// 宽字符分类/转换辅助
mod wctrans;

// 宽字符宽度
mod wcswidth;
mod wcwidth;

// 宽字符分类
mod iswcntrl;
mod iswctype;
mod iswdigit;
mod iswgraph;
mod iswlower;
mod iswprint;
mod iswalnum;
mod iswalpha;
mod iswblank;
mod iswpunct;
mod iswspace;
mod iswupper;
mod iswxdigit;

// 类型定义 (供 crate 内部使用)
pub use iswctype::{
    WCTYPE_ALNUM, WCTYPE_ALPHA, WCTYPE_BLANK, WCTYPE_CNTRL,
    WCTYPE_DIGIT, WCTYPE_GRAPH, WCTYPE_LOWER, WCTYPE_PRINT,
    WCTYPE_PUNCT, WCTYPE_SPACE, WCTYPE_UPPER, WCTYPE_XDIGIT,
};

// 重导出依赖类型，供 crate 内部使用
pub(crate) use rusl_internal::libc::__locale_struct;

// 底层 locale 辅助函数公开导出
pub(crate) use __ctype_b_loc::__ctype_b_loc;
pub use __ctype_get_mb_cur_max::__ctype_get_mb_cur_max;
pub(crate) use __ctype_tolower_loc::__ctype_tolower_loc;
pub(crate) use __ctype_toupper_loc::__ctype_toupper_loc;

// 字节字符分类公开导出
pub use isalpha::{isalpha, isalpha_l};
pub(crate) use isalpha::__isalpha_l;
pub use isascii::isascii;
pub use isblank::{isblank, isblank_l};
pub use iscntrl::{iscntrl, iscntrl_l};
pub use isdigit::{isdigit, isdigit_l};
pub use isgraph::{isgraph, isgraph_l};
pub use islower::{islower, islower_l};
pub use isprint::{isprint, isprint_l};
pub use ispunct::{ispunct, ispunct_l};
pub use isspace::{isspace, isspace_l};
pub use isupper::{isupper, isupper_l};
pub use isxdigit::{isxdigit, isxdigit_l};
pub use toascii::toascii;

// 宽字符分类公开导出
pub use iswcntrl::{iswcntrl, iswcntrl_l};
pub use iswctype::{iswctype, iswctype_l, wctype, wctype_l};
pub use iswdigit::{iswdigit, iswdigit_l};
pub use iswgraph::{iswgraph, iswgraph_l};
pub use iswlower::{iswlower, iswlower_l};
pub use iswprint::{iswprint, iswprint_l};
pub use iswalnum::{iswalnum, iswalnum_l};
pub use iswalpha::{iswalpha, iswalpha_l};
pub use iswblank::{iswblank, iswblank_l};
pub use iswpunct::{iswpunct, iswpunct_l};
pub use iswspace::{iswspace, iswspace_l};
pub use iswupper::{iswupper, iswupper_l};
pub use iswxdigit::{iswxdigit, iswxdigit_l};

// 大小写转换 (字节) 公开导出
pub use tolower::{tolower, tolower_l};
pub use toupper::{toupper, toupper_l};

// 大小写转换 (宽字符) 公开导出
pub use towctrans::{towlower, towupper, towlower_l, towupper_l};

// 宽字符分类/转换辅助公开导出
pub use wctrans::{wctrans, towctrans, wctrans_l, towctrans_l};

// 宽字符宽度公开导出
pub use wcswidth::wcswidth;
pub use wcwidth::wcwidth;