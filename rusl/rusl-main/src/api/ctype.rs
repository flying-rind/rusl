//! Ctype — 字符分类、大小写转换、宽字符辅助、locale 辅助函数

use core::ffi::{c_int, c_char};
use crate::api::types::{wchar_t, wint_t, wctype_t, wctrans_t, locale_t};


// ---------- WCTYPE constants ----------

pub const WCTYPE_ALNUM: wctype_t  = 1;
pub const WCTYPE_ALPHA: wctype_t  = 2;
pub const WCTYPE_BLANK: wctype_t  = 3;
pub const WCTYPE_CNTRL: wctype_t  = 4;
pub const WCTYPE_DIGIT: wctype_t  = 5;
pub const WCTYPE_GRAPH: wctype_t  = 6;
pub const WCTYPE_LOWER: wctype_t  = 7;
pub const WCTYPE_PRINT: wctype_t  = 8;
pub const WCTYPE_PUNCT: wctype_t  = 9;
pub const WCTYPE_SPACE: wctype_t  = 10;
pub const WCTYPE_UPPER: wctype_t  = 11;
pub const WCTYPE_XDIGIT: wctype_t = 12;

// ---------- internal FFI declarations ----------

// -- byte classification --
extern "C" {
    #[link_name = "isalnum"]
    fn musl_isalnum(c: c_int) -> c_int;
    #[link_name = "isalnum_l"]
    fn musl_isalnum_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isalpha"]
    fn musl_isalpha(c: c_int) -> c_int;
    #[link_name = "isalpha_l"]
    fn musl_isalpha_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isascii"]
    fn musl_isascii(c: c_int) -> c_int;
    #[link_name = "isblank"]
    fn musl_isblank(c: c_int) -> c_int;
    #[link_name = "isblank_l"]
    fn musl_isblank_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "iscntrl"]
    fn musl_iscntrl(c: c_int) -> c_int;
    #[link_name = "iscntrl_l"]
    fn musl_iscntrl_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isdigit"]
    fn musl_isdigit(c: c_int) -> c_int;
    #[link_name = "isdigit_l"]
    fn musl_isdigit_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isgraph"]
    fn musl_isgraph(c: c_int) -> c_int;
    #[link_name = "isgraph_l"]
    fn musl_isgraph_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "islower"]
    fn musl_islower(c: c_int) -> c_int;
    #[link_name = "islower_l"]
    fn musl_islower_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isprint"]
    fn musl_isprint(c: c_int) -> c_int;
    #[link_name = "isprint_l"]
    fn musl_isprint_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "ispunct"]
    fn musl_ispunct(c: c_int) -> c_int;
    #[link_name = "ispunct_l"]
    fn musl_ispunct_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isspace"]
    fn musl_isspace(c: c_int) -> c_int;
    #[link_name = "isspace_l"]
    fn musl_isspace_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isupper"]
    fn musl_isupper(c: c_int) -> c_int;
    #[link_name = "isupper_l"]
    fn musl_isupper_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "isxdigit"]
    fn musl_isxdigit(c: c_int) -> c_int;
    #[link_name = "isxdigit_l"]
    fn musl_isxdigit_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "toascii"]
    fn musl_toascii(c: c_int) -> c_int;
}

// -- byte case conversion --
extern "C" {
    #[link_name = "tolower"]
    fn musl_tolower(c: c_int) -> c_int;
    #[link_name = "tolower_l"]
    fn musl_tolower_l(c: c_int, loc: locale_t) -> c_int;
    #[link_name = "toupper"]
    fn musl_toupper(c: c_int) -> c_int;
    #[link_name = "toupper_l"]
    fn musl_toupper_l(c: c_int, loc: locale_t) -> c_int;
}

// -- wide char classification --
extern "C" {
    #[link_name = "iswalnum"]
    fn musl_iswalnum(wc: wint_t) -> c_int;
    #[link_name = "iswalnum_l"]
    fn musl_iswalnum_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswalpha"]
    fn musl_iswalpha(wc: wint_t) -> c_int;
    #[link_name = "iswalpha_l"]
    fn musl_iswalpha_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswblank"]
    fn musl_iswblank(wc: wint_t) -> c_int;
    #[link_name = "iswblank_l"]
    fn musl_iswblank_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswcntrl"]
    fn musl_iswcntrl(wc: wint_t) -> c_int;
    #[link_name = "iswcntrl_l"]
    fn musl_iswcntrl_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswctype"]
    fn musl_iswctype(wc: wint_t, desc: wctype_t) -> c_int;
    #[link_name = "iswctype_l"]
    fn musl_iswctype_l(wc: wint_t, desc: wctype_t, loc: locale_t) -> c_int;
    #[link_name = "iswdigit"]
    fn musl_iswdigit(wc: wint_t) -> c_int;
    #[link_name = "iswdigit_l"]
    fn musl_iswdigit_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswgraph"]
    fn musl_iswgraph(wc: wint_t) -> c_int;
    #[link_name = "iswgraph_l"]
    fn musl_iswgraph_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswlower"]
    fn musl_iswlower(wc: wint_t) -> c_int;
    #[link_name = "iswlower_l"]
    fn musl_iswlower_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswprint"]
    fn musl_iswprint(wc: wint_t) -> c_int;
    #[link_name = "iswprint_l"]
    fn musl_iswprint_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswpunct"]
    fn musl_iswpunct(wc: wint_t) -> c_int;
    #[link_name = "iswpunct_l"]
    fn musl_iswpunct_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswspace"]
    fn musl_iswspace(wc: wint_t) -> c_int;
    #[link_name = "iswspace_l"]
    fn musl_iswspace_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswupper"]
    fn musl_iswupper(wc: wint_t) -> c_int;
    #[link_name = "iswupper_l"]
    fn musl_iswupper_l(wc: wint_t, loc: locale_t) -> c_int;
    #[link_name = "iswxdigit"]
    fn musl_iswxdigit(wc: wint_t) -> c_int;
    #[link_name = "iswxdigit_l"]
    fn musl_iswxdigit_l(wc: wint_t, loc: locale_t) -> c_int;
}

// -- wide char case conversion --
extern "C" {
    #[link_name = "towlower"]
    fn musl_towlower(wc: wint_t) -> wint_t;
    #[link_name = "towlower_l"]
    fn musl_towlower_l(wc: wint_t, loc: locale_t) -> wint_t;
    #[link_name = "towupper"]
    fn musl_towupper(wc: wint_t) -> wint_t;
    #[link_name = "towupper_l"]
    fn musl_towupper_l(wc: wint_t, loc: locale_t) -> wint_t;
}

// -- wide char helpers --
extern "C" {
    #[link_name = "wcwidth"]
    fn musl_wcwidth(wc: wchar_t) -> c_int;
    #[link_name = "wcswidth"]
    fn musl_wcswidth(ws: *const wchar_t, n: usize) -> c_int;
    #[link_name = "wctype"]
    fn musl_wctype(class: *const c_char) -> wctype_t;
    #[link_name = "wctype_l"]
    fn musl_wctype_l(class: *const c_char, loc: locale_t) -> wctype_t;
    #[link_name = "wctrans"]
    fn musl_wctrans(class: *const c_char) -> wctrans_t;
    #[link_name = "wctrans_l"]
    fn musl_wctrans_l(class: *const c_char, loc: locale_t) -> wctrans_t;
    #[link_name = "towctrans"]
    fn musl_towctrans(wc: wint_t, trans: wctrans_t) -> wint_t;
    #[link_name = "towctrans_l"]
    fn musl_towctrans_l(wc: wint_t, trans: wctrans_t, loc: locale_t) -> wint_t;
}

// -- locale helpers --
extern "C" {
    #[link_name = "__ctype_get_mb_cur_max"]
    fn musl___ctype_get_mb_cur_max() -> usize;
}

// ---------- safe public wrappers ----------

// -- byte classification --
pub extern "C" fn isalnum(c: c_int) -> c_int              { unsafe { musl_isalnum(c) } }
pub extern "C" fn isalnum_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isalnum_l(c, loc) } }
pub extern "C" fn isalpha(c: c_int) -> c_int              { unsafe { musl_isalpha(c) } }
pub extern "C" fn isalpha_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isalpha_l(c, loc) } }
pub extern "C" fn isascii(c: c_int) -> c_int              { unsafe { musl_isascii(c) } }
pub extern "C" fn isblank(c: c_int) -> c_int              { unsafe { musl_isblank(c) } }
pub extern "C" fn isblank_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isblank_l(c, loc) } }
pub extern "C" fn iscntrl(c: c_int) -> c_int              { unsafe { musl_iscntrl(c) } }
pub extern "C" fn iscntrl_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_iscntrl_l(c, loc) } }
pub extern "C" fn isdigit(c: c_int) -> c_int              { unsafe { musl_isdigit(c) } }
pub extern "C" fn isdigit_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isdigit_l(c, loc) } }
pub extern "C" fn isgraph(c: c_int) -> c_int              { unsafe { musl_isgraph(c) } }
pub extern "C" fn isgraph_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isgraph_l(c, loc) } }
pub extern "C" fn islower(c: c_int) -> c_int              { unsafe { musl_islower(c) } }
pub extern "C" fn islower_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_islower_l(c, loc) } }
pub extern "C" fn isprint(c: c_int) -> c_int              { unsafe { musl_isprint(c) } }
pub extern "C" fn isprint_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isprint_l(c, loc) } }
pub extern "C" fn ispunct(c: c_int) -> c_int              { unsafe { musl_ispunct(c) } }
pub extern "C" fn ispunct_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_ispunct_l(c, loc) } }
pub extern "C" fn isspace(c: c_int) -> c_int              { unsafe { musl_isspace(c) } }
pub extern "C" fn isspace_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isspace_l(c, loc) } }
pub extern "C" fn isupper(c: c_int) -> c_int              { unsafe { musl_isupper(c) } }
pub extern "C" fn isupper_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isupper_l(c, loc) } }
pub extern "C" fn isxdigit(c: c_int) -> c_int             { unsafe { musl_isxdigit(c) } }
pub extern "C" fn isxdigit_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_isxdigit_l(c, loc) } }
pub extern "C" fn toascii(c: c_int) -> c_int              { unsafe { musl_toascii(c) } }

// -- byte case conversion --
pub extern "C" fn tolower(c: c_int) -> c_int              { unsafe { musl_tolower(c) } }
pub extern "C" fn tolower_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_tolower_l(c, loc) } }
pub extern "C" fn toupper(c: c_int) -> c_int              { unsafe { musl_toupper(c) } }
pub extern "C" fn toupper_l(c: c_int, loc: locale_t) -> c_int { unsafe { musl_toupper_l(c, loc) } }

// -- wide char classification --
pub extern "C" fn iswalnum(wc: wint_t) -> c_int                 { unsafe { musl_iswalnum(wc) } }
pub extern "C" fn iswalnum_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswalnum_l(wc, loc) } }
pub extern "C" fn iswalpha(wc: wint_t) -> c_int                 { unsafe { musl_iswalpha(wc) } }
pub extern "C" fn iswalpha_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswalpha_l(wc, loc) } }
pub extern "C" fn iswblank(wc: wint_t) -> c_int                 { unsafe { musl_iswblank(wc) } }
pub extern "C" fn iswblank_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswblank_l(wc, loc) } }
pub extern "C" fn iswcntrl(wc: wint_t) -> c_int                 { unsafe { musl_iswcntrl(wc) } }
pub extern "C" fn iswcntrl_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswcntrl_l(wc, loc) } }
pub extern "C" fn iswctype(wc: wint_t, desc: wctype_t) -> c_int { unsafe { musl_iswctype(wc, desc) } }
pub extern "C" fn iswctype_l(wc: wint_t, desc: wctype_t, loc: locale_t) -> c_int { unsafe { musl_iswctype_l(wc, desc, loc) } }
pub extern "C" fn iswdigit(wc: wint_t) -> c_int                 { unsafe { musl_iswdigit(wc) } }
pub extern "C" fn iswdigit_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswdigit_l(wc, loc) } }
pub extern "C" fn iswgraph(wc: wint_t) -> c_int                 { unsafe { musl_iswgraph(wc) } }
pub extern "C" fn iswgraph_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswgraph_l(wc, loc) } }
pub extern "C" fn iswlower(wc: wint_t) -> c_int                 { unsafe { musl_iswlower(wc) } }
pub extern "C" fn iswlower_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswlower_l(wc, loc) } }
pub extern "C" fn iswprint(wc: wint_t) -> c_int                 { unsafe { musl_iswprint(wc) } }
pub extern "C" fn iswprint_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswprint_l(wc, loc) } }
pub extern "C" fn iswpunct(wc: wint_t) -> c_int                 { unsafe { musl_iswpunct(wc) } }
pub extern "C" fn iswpunct_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswpunct_l(wc, loc) } }
pub extern "C" fn iswspace(wc: wint_t) -> c_int                 { unsafe { musl_iswspace(wc) } }
pub extern "C" fn iswspace_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswspace_l(wc, loc) } }
pub extern "C" fn iswupper(wc: wint_t) -> c_int                 { unsafe { musl_iswupper(wc) } }
pub extern "C" fn iswupper_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswupper_l(wc, loc) } }
pub extern "C" fn iswxdigit(wc: wint_t) -> c_int                { unsafe { musl_iswxdigit(wc) } }
pub extern "C" fn iswxdigit_l(wc: wint_t, loc: locale_t) -> c_int { unsafe { musl_iswxdigit_l(wc, loc) } }

// -- wide char case conversion --
pub extern "C" fn towlower(wc: wint_t) -> wint_t                     { unsafe { musl_towlower(wc) } }
pub extern "C" fn towlower_l(wc: wint_t, loc: locale_t) -> wint_t    { unsafe { musl_towlower_l(wc, loc) } }
pub extern "C" fn towupper(wc: wint_t) -> wint_t                     { unsafe { musl_towupper(wc) } }
pub extern "C" fn towupper_l(wc: wint_t, loc: locale_t) -> wint_t    { unsafe { musl_towupper_l(wc, loc) } }

// -- wide char helpers --
pub extern "C" fn wcwidth(wc: wchar_t) -> c_int                      { unsafe { musl_wcwidth(wc) } }
pub extern "C" fn wcswidth(ws: *const wchar_t, n: usize) -> c_int    { unsafe { musl_wcswidth(ws, n) } }
pub extern "C" fn wctype(class: *const c_char) -> wctype_t            { unsafe { musl_wctype(class) } }
pub extern "C" fn wctype_l(class: *const c_char, loc: locale_t) -> wctype_t { unsafe { musl_wctype_l(class, loc) } }
pub extern "C" fn wctrans(class: *const c_char) -> wctrans_t          { unsafe { musl_wctrans(class) } }
pub extern "C" fn wctrans_l(class: *const c_char, loc: locale_t) -> wctrans_t { unsafe { musl_wctrans_l(class, loc) } }
pub extern "C" fn towctrans(wc: wint_t, trans: wctrans_t) -> wint_t  { unsafe { musl_towctrans(wc, trans) } }
pub extern "C" fn towctrans_l(wc: wint_t, trans: wctrans_t, loc: locale_t) -> wint_t { unsafe { musl_towctrans_l(wc, trans, loc) } }

// -- locale helpers --
pub extern "C" fn __ctype_get_mb_cur_max() -> usize           { unsafe { musl___ctype_get_mb_cur_max() } }