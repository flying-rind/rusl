//! String/memory — 内存/字符串/宽字符操作

use core::ffi::{c_char, c_int, c_void};

// ---------- internal FFI declarations ----------

// -- BSD / legacy --
extern "C" {
    #[link_name = "bcmp"]
    fn musl_bcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    #[link_name = "bcopy"]
    fn musl_bcopy(src: *const c_void, dst: *mut c_void, n: usize);
    #[link_name = "bzero"]
    fn musl_bzero(s: *mut c_void, n: usize);
    #[link_name = "explicit_bzero"]
    fn musl_explicit_bzero(s: *mut c_void, n: usize);
    #[link_name = "index"]
    fn musl_index(s: *const c_char, c: c_int) -> *mut c_char;
    #[link_name = "rindex"]
    fn musl_rindex(s: *const c_char, c: c_int) -> *mut c_char;
}

// -- memory operations --
extern "C" {
    #[link_name = "memccpy"]
    fn musl_memccpy(dest: *mut c_void, src: *const c_void, c: c_int, n: usize) -> *mut c_void;
    #[link_name = "memchr"]
    fn musl_memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    #[link_name = "memcmp"]
    fn musl_memcmp(vl: *const c_void, vr: *const c_void, n: usize) -> c_int;
    #[link_name = "memcpy"]
    fn musl_memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    #[link_name = "memmem"]
    fn musl_memmem(haystack: *const c_void, haystacklen: usize, needle: *const c_void, needlelen: usize) -> *mut c_void;
    #[link_name = "memmove"]
    fn musl_memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    #[link_name = "mempcpy"]
    fn musl_mempcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    #[link_name = "memrchr"]
    fn musl_memrchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    #[link_name = "memset"]
    fn musl_memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    #[link_name = "swab"]
    fn musl_swab(src: *const c_void, dst: *mut c_void, n: isize);
}

// -- byte string operations --
extern "C" {
    #[link_name = "stpcpy"]
    fn musl_stpcpy(d: *mut c_char, s: *const c_char) -> *mut c_char;
    #[link_name = "stpncpy"]
    fn musl_stpncpy(d: *mut c_char, s: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "strcasecmp"]
    fn musl_strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int;
    #[link_name = "strcasestr"]
    fn musl_strcasestr(h: *const c_char, n: *const c_char) -> *mut c_char;
    #[link_name = "strcat"]
    fn musl_strcat(d: *mut c_char, s: *const c_char) -> *mut c_char;
    #[link_name = "strchr"]
    fn musl_strchr(s: *const c_char, c: c_int) -> *mut c_char;
    #[link_name = "strchrnul"]
    fn musl_strchrnul(s: *const c_char, c: c_int) -> *mut c_char;
    #[link_name = "strcmp"]
    fn musl_strcmp(l: *const c_char, r: *const c_char) -> c_int;
    #[link_name = "strcpy"]
    fn musl_strcpy(d: *mut c_char, s: *const c_char) -> *mut c_char;
    #[link_name = "strcspn"]
    fn musl_strcspn(s: *const c_char, c: *const c_char) -> usize;
    #[link_name = "strdup"]
    fn musl_strdup(s: *const c_char) -> *mut c_char;
    #[link_name = "strerror_r"]
    fn musl_strerror_r(err: c_int, buf: *mut c_char, buflen: usize) -> c_int;
    #[link_name = "strlcat"]
    fn musl_strlcat(d: *mut c_char, s: *const c_char, n: usize) -> usize;
    #[link_name = "strlcpy"]
    fn musl_strlcpy(d: *mut c_char, s: *const c_char, n: usize) -> usize;
    #[link_name = "strlen"]
    fn musl_strlen(s: *const c_char) -> usize;
    #[link_name = "strncasecmp"]
    fn musl_strncasecmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    #[link_name = "strncat"]
    fn musl_strncat(d: *mut c_char, s: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "strncmp"]
    fn musl_strncmp(l: *const c_char, r: *const c_char, n: usize) -> c_int;
    #[link_name = "strncpy"]
    fn musl_strncpy(d: *mut c_char, s: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "strndup"]
    fn musl_strndup(s: *const c_char, n: usize) -> *mut c_char;
    #[link_name = "strnlen"]
    fn musl_strnlen(s: *const c_char, n: usize) -> usize;
    #[link_name = "strpbrk"]
    fn musl_strpbrk(s: *const c_char, b: *const c_char) -> *mut c_char;
    #[link_name = "strrchr"]
    fn musl_strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    #[link_name = "strsep"]
    fn musl_strsep(s: *mut *mut c_char, delim: *const c_char) -> *mut c_char;
    #[link_name = "strsignal"]
    fn musl_strsignal(signum: c_int) -> *mut c_char;
    #[link_name = "strspn"]
    fn musl_strspn(s: *const c_char, c: *const c_char) -> usize;
    #[link_name = "strstr"]
    fn musl_strstr(h: *const c_char, n: *const c_char) -> *mut c_char;
    #[link_name = "strtok"]
    fn musl_strtok(s: *mut c_char, delim: *const c_char) -> *mut c_char;
    #[link_name = "strtok_r"]
    fn musl_strtok_r(s: *mut c_char, delim: *const c_char, saveptr: *mut *mut c_char) -> *mut c_char;
    #[link_name = "strverscmp"]
    fn musl_strverscmp(s1: *const c_char, s2: *const c_char) -> c_int;
}

// -- wide character operations --
extern "C" {
    #[link_name = "wcpcpy"]
    fn musl_wcpcpy(d: *mut u32, s: *const u32) -> *mut u32;
    #[link_name = "wcpncpy"]
    fn musl_wcpncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
    #[link_name = "wcscasecmp"]
    fn musl_wcscasecmp(l: *const u32, r: *const u32) -> c_int;
    #[link_name = "wcscasecmp_l"]
    fn musl_wcscasecmp_l(l: *const u32, r: *const u32, loc: *mut c_void) -> c_int;
    #[link_name = "wcscat"]
    fn musl_wcscat(d: *mut u32, s: *const u32) -> *mut u32;
    #[link_name = "wcschr"]
    fn musl_wcschr(s: *const u32, c: u32) -> *mut u32;
    #[link_name = "wcscmp"]
    fn musl_wcscmp(l: *const u32, r: *const u32) -> c_int;
    #[link_name = "wcscpy"]
    fn musl_wcscpy(d: *mut u32, s: *const u32) -> *mut u32;
    #[link_name = "wcscspn"]
    fn musl_wcscspn(s: *const u32, c: *const u32) -> usize;
    #[link_name = "wcsdup"]
    fn musl_wcsdup(s: *const u32) -> *mut u32;
    #[link_name = "wcslen"]
    fn musl_wcslen(s: *const u32) -> usize;
    #[link_name = "wcsncasecmp"]
    fn musl_wcsncasecmp(l: *const u32, r: *const u32, n: usize) -> c_int;
    #[link_name = "wcsncasecmp_l"]
    fn musl_wcsncasecmp_l(l: *const u32, r: *const u32, n: usize, loc: *mut c_void) -> c_int;
    #[link_name = "wcsncat"]
    fn musl_wcsncat(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
    #[link_name = "wcsncmp"]
    fn musl_wcsncmp(l: *const u32, r: *const u32, n: usize) -> c_int;
    #[link_name = "wcsncpy"]
    fn musl_wcsncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
    #[link_name = "wcsnlen"]
    fn musl_wcsnlen(s: *const u32, n: usize) -> usize;
    #[link_name = "wcspbrk"]
    fn musl_wcspbrk(s: *const u32, b: *const u32) -> *mut u32;
    #[link_name = "wcsrchr"]
    fn musl_wcsrchr(s: *const u32, c: u32) -> *mut u32;
    #[link_name = "wcsspn"]
    fn musl_wcsspn(s: *const u32, c: *const u32) -> usize;
    #[link_name = "wcsstr"]
    fn musl_wcsstr(h: *const u32, n: *const u32) -> *mut u32;
    #[link_name = "wcstok"]
    fn musl_wcstok(s: *mut u32, delim: *const u32, saveptr: *mut *mut u32) -> *mut u32;
    #[link_name = "wcswcs"]
    fn musl_wcswcs(h: *const u32, n: *const u32) -> *mut u32;
    #[link_name = "wmemchr"]
    fn musl_wmemchr(s: *const u32, c: u32, n: usize) -> *mut u32;
    #[link_name = "wmemcmp"]
    fn musl_wmemcmp(l: *const u32, r: *const u32, n: usize) -> c_int;
    #[link_name = "wmemcpy"]
    fn musl_wmemcpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
    #[link_name = "wmemmove"]
    fn musl_wmemmove(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
    #[link_name = "wmemset"]
    fn musl_wmemset(s: *mut u32, c: u32, n: usize) -> *mut u32;
}

// ---------- safe public wrappers ----------

// -- BSD / legacy --
pub extern "C" fn bcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int     { unsafe { musl_bcmp(s1, s2, n) } }
pub extern "C" fn bcopy(src: *const c_void, dst: *mut c_void, n: usize)              { unsafe { musl_bcopy(src, dst, n) } }
pub extern "C" fn bzero(s: *mut c_void, n: usize)                                    { unsafe { musl_bzero(s, n) } }
pub extern "C" fn explicit_bzero(s: *mut c_void, n: usize)                           { unsafe { musl_explicit_bzero(s, n) } }
pub extern "C" fn index(s: *const c_char, c: c_int) -> *mut c_char                   { unsafe { musl_index(s, c) } }
pub extern "C" fn rindex(s: *const c_char, c: c_int) -> *mut c_char                  { unsafe { musl_rindex(s, c) } }

// -- memory operations --
pub extern "C" fn memccpy(dest: *mut c_void, src: *const c_void, c: c_int, n: usize) -> *mut c_void { unsafe { musl_memccpy(dest, src, c, n) } }
pub extern "C" fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void        { unsafe { musl_memchr(s, c, n) } }
pub extern "C" fn memcmp(vl: *const c_void, vr: *const c_void, n: usize) -> c_int    { unsafe { musl_memcmp(vl, vr, n) } }
pub extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void { unsafe { musl_memcpy(dest, src, n) } }
pub extern "C" fn memmem(haystack: *const c_void, haystacklen: usize, needle: *const c_void, needlelen: usize) -> *mut c_void { unsafe { musl_memmem(haystack, haystacklen, needle, needlelen) } }
pub extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void { unsafe { musl_memmove(dest, src, n) } }
pub extern "C" fn mempcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void { unsafe { musl_mempcpy(dest, src, n) } }
pub extern "C" fn memrchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void       { unsafe { musl_memrchr(s, c, n) } }
pub extern "C" fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void          { unsafe { musl_memset(s, c, n) } }
pub extern "C" fn swab(src: *const c_void, dst: *mut c_void, n: isize)               { unsafe { musl_swab(src, dst, n) } }

// -- byte string operations --
pub extern "C" fn stpcpy(d: *mut c_char, s: *const c_char) -> *mut c_char            { unsafe { musl_stpcpy(d, s) } }
pub extern "C" fn stpncpy(d: *mut c_char, s: *const c_char, n: usize) -> *mut c_char { unsafe { musl_stpncpy(d, s, n) } }
pub extern "C" fn strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int          { unsafe { musl_strcasecmp(s1, s2) } }
pub extern "C" fn strcasestr(h: *const c_char, n: *const c_char) -> *mut c_char      { unsafe { musl_strcasestr(h, n) } }
pub extern "C" fn strcat(d: *mut c_char, s: *const c_char) -> *mut c_char            { unsafe { musl_strcat(d, s) } }
pub extern "C" fn strchr(s: *const c_char, c: c_int) -> *mut c_char                  { unsafe { musl_strchr(s, c) } }
pub extern "C" fn strchrnul(s: *const c_char, c: c_int) -> *mut c_char               { unsafe { musl_strchrnul(s, c) } }
pub extern "C" fn strcmp(l: *const c_char, r: *const c_char) -> c_int                { unsafe { musl_strcmp(l, r) } }
pub extern "C" fn strcpy(d: *mut c_char, s: *const c_char) -> *mut c_char            { unsafe { musl_strcpy(d, s) } }
pub extern "C" fn strcspn(s: *const c_char, c: *const c_char) -> usize               { unsafe { musl_strcspn(s, c) } }
pub extern "C" fn strdup(s: *const c_char) -> *mut c_char                            { unsafe { musl_strdup(s) } }
pub extern "C" fn strerror_r(err: c_int, buf: *mut c_char, buflen: usize) -> c_int   { unsafe { musl_strerror_r(err, buf, buflen) } }
pub extern "C" fn strlcat(d: *mut c_char, s: *const c_char, n: usize) -> usize       { unsafe { musl_strlcat(d, s, n) } }
pub extern "C" fn strlcpy(d: *mut c_char, s: *const c_char, n: usize) -> usize       { unsafe { musl_strlcpy(d, s, n) } }
pub extern "C" fn strlen(s: *const c_char) -> usize                                  { unsafe { musl_strlen(s) } }
pub extern "C" fn strncasecmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int { unsafe { musl_strncasecmp(s1, s2, n) } }
pub extern "C" fn strncat(d: *mut c_char, s: *const c_char, n: usize) -> *mut c_char { unsafe { musl_strncat(d, s, n) } }
pub extern "C" fn strncmp(l: *const c_char, r: *const c_char, n: usize) -> c_int     { unsafe { musl_strncmp(l, r, n) } }
pub extern "C" fn strncpy(d: *mut c_char, s: *const c_char, n: usize) -> *mut c_char { unsafe { musl_strncpy(d, s, n) } }
pub extern "C" fn strndup(s: *const c_char, n: usize) -> *mut c_char                 { unsafe { musl_strndup(s, n) } }
pub extern "C" fn strnlen(s: *const c_char, n: usize) -> usize                       { unsafe { musl_strnlen(s, n) } }
pub extern "C" fn strpbrk(s: *const c_char, b: *const c_char) -> *mut c_char         { unsafe { musl_strpbrk(s, b) } }
pub extern "C" fn strrchr(s: *const c_char, c: c_int) -> *mut c_char                 { unsafe { musl_strrchr(s, c) } }
pub extern "C" fn strsep(s: *mut *mut c_char, delim: *const c_char) -> *mut c_char   { unsafe { musl_strsep(s, delim) } }
pub extern "C" fn strsignal(signum: c_int) -> *mut c_char                            { unsafe { musl_strsignal(signum) } }
pub extern "C" fn strspn(s: *const c_char, c: *const c_char) -> usize                { unsafe { musl_strspn(s, c) } }
pub extern "C" fn strstr(h: *const c_char, n: *const c_char) -> *mut c_char          { unsafe { musl_strstr(h, n) } }
pub extern "C" fn strtok(s: *mut c_char, delim: *const c_char) -> *mut c_char        { unsafe { musl_strtok(s, delim) } }
pub extern "C" fn strtok_r(s: *mut c_char, delim: *const c_char, saveptr: *mut *mut c_char) -> *mut c_char { unsafe { musl_strtok_r(s, delim, saveptr) } }
pub extern "C" fn strverscmp(s1: *const c_char, s2: *const c_char) -> c_int          { unsafe { musl_strverscmp(s1, s2) } }

// -- wide character operations --
pub extern "C" fn wcpcpy(d: *mut u32, s: *const u32) -> *mut u32                     { unsafe { musl_wcpcpy(d, s) } }
pub extern "C" fn wcpncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32          { unsafe { musl_wcpncpy(d, s, n) } }
pub extern "C" fn wcscasecmp(l: *const u32, r: *const u32) -> c_int                  { unsafe { musl_wcscasecmp(l, r) } }
pub extern "C" fn wcscasecmp_l(l: *const u32, r: *const u32, loc: *mut c_void) -> c_int { unsafe { musl_wcscasecmp_l(l, r, loc) } }
pub extern "C" fn wcscat(d: *mut u32, s: *const u32) -> *mut u32                     { unsafe { musl_wcscat(d, s) } }
pub extern "C" fn wcschr(s: *const u32, c: u32) -> *mut u32                          { unsafe { musl_wcschr(s, c) } }
pub extern "C" fn wcscmp(l: *const u32, r: *const u32) -> c_int                      { unsafe { musl_wcscmp(l, r) } }
pub extern "C" fn wcscpy(d: *mut u32, s: *const u32) -> *mut u32                     { unsafe { musl_wcscpy(d, s) } }
pub extern "C" fn wcscspn(s: *const u32, c: *const u32) -> usize                     { unsafe { musl_wcscspn(s, c) } }
pub extern "C" fn wcsdup(s: *const u32) -> *mut u32                                  { unsafe { musl_wcsdup(s) } }
pub extern "C" fn wcslen(s: *const u32) -> usize                                     { unsafe { musl_wcslen(s) } }
pub extern "C" fn wcsncasecmp(l: *const u32, r: *const u32, n: usize) -> c_int       { unsafe { musl_wcsncasecmp(l, r, n) } }
pub extern "C" fn wcsncasecmp_l(l: *const u32, r: *const u32, n: usize, loc: *mut c_void) -> c_int { unsafe { musl_wcsncasecmp_l(l, r, n, loc) } }
pub extern "C" fn wcsncat(d: *mut u32, s: *const u32, n: usize) -> *mut u32          { unsafe { musl_wcsncat(d, s, n) } }
pub extern "C" fn wcsncmp(l: *const u32, r: *const u32, n: usize) -> c_int           { unsafe { musl_wcsncmp(l, r, n) } }
pub extern "C" fn wcsncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32          { unsafe { musl_wcsncpy(d, s, n) } }
pub extern "C" fn wcsnlen(s: *const u32, n: usize) -> usize                          { unsafe { musl_wcsnlen(s, n) } }
pub extern "C" fn wcspbrk(s: *const u32, b: *const u32) -> *mut u32                  { unsafe { musl_wcspbrk(s, b) } }
pub extern "C" fn wcsrchr(s: *const u32, c: u32) -> *mut u32                         { unsafe { musl_wcsrchr(s, c) } }
pub extern "C" fn wcsspn(s: *const u32, c: *const u32) -> usize                      { unsafe { musl_wcsspn(s, c) } }
pub extern "C" fn wcsstr(h: *const u32, n: *const u32) -> *mut u32                   { unsafe { musl_wcsstr(h, n) } }
pub extern "C" fn wcstok(s: *mut u32, delim: *const u32, saveptr: *mut *mut u32) -> *mut u32 { unsafe { musl_wcstok(s, delim, saveptr) } }
pub extern "C" fn wcswcs(h: *const u32, n: *const u32) -> *mut u32                   { unsafe { musl_wcswcs(h, n) } }
pub extern "C" fn wmemchr(s: *const u32, c: u32, n: usize) -> *mut u32               { unsafe { musl_wmemchr(s, c, n) } }
pub extern "C" fn wmemcmp(l: *const u32, r: *const u32, n: usize) -> c_int           { unsafe { musl_wmemcmp(l, r, n) } }
pub extern "C" fn wmemcpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32          { unsafe { musl_wmemcpy(d, s, n) } }
pub extern "C" fn wmemmove(d: *mut u32, s: *const u32, n: usize) -> *mut u32         { unsafe { musl_wmemmove(d, s, n) } }
pub extern "C" fn wmemset(s: *mut u32, c: u32, n: usize) -> *mut u32                 { unsafe { musl_wmemset(s, c, n) } }