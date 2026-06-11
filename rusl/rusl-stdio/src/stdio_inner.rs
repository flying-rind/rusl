//! stdio — 标准 I/O 实现。
//! 对应 musl src/stdio/ 目录。

#![allow(dead_code, unused_imports)]

pub(crate) mod stdio_impl;

// ============================================================
// 基础设施（实现在 external C 包装器中，如 snprintf_wrapper.c）
// ============================================================
mod snprintf;
// snprintf is provided by C wrapper (snprintf_wrapper.c) compiled in build.rs

// ============================================================
// 核心格式化引擎
// ============================================================
mod vfprintf;
mod vfwprintf;
mod vsnprintf;
mod vfscanf;
mod vfwscanf;

pub use vfprintf::vfprintf;
pub use vfwprintf::vfwprintf;
pub use vsnprintf::vsnprintf;
pub use vfscanf::vfscanf;
pub use vfwscanf::vfwscanf;

// ============================================================
// 底层 I/O 原语
// ============================================================
mod __toread;
mod __towrite;
mod __uflow;
mod __overflow;
mod __lockfile;

pub(crate) use __toread::*;
pub(crate) use __towrite::*;
pub(crate) use __uflow::*;
pub(crate) use __overflow::*;
pub(crate) use __lockfile::*;

// ============================================================
// 内部 FILE 操作
// ============================================================
mod __fclose_ca;
mod __fdopen;
mod __fmodeflags;
mod __fopen_rb_ca;
mod __stdio_close;
mod __stdio_exit;
mod __stdio_read;
mod __stdio_seek;
mod __stdio_write;
mod __stdout_write;

pub(crate) use __fclose_ca::*;
pub(crate) use __fdopen::*;
pub(crate) use __fmodeflags::*;
pub(crate) use __fopen_rb_ca::*;
pub(crate) use __stdio_close::*;
pub(crate) use __stdio_exit::*;
pub(crate) use __stdio_read::*;
pub(crate) use __stdio_seek::*;
pub(crate) use __stdio_write::*;
pub(crate) use __stdout_write::*;

// ============================================================
// 文件打开/关闭
// ============================================================
mod fopen;
mod fclose;
mod fopencookie;

pub use fopen::*;
pub use fclose::*;
pub use fopencookie::*;

// ============================================================
// 文件 I/O
// ============================================================
mod fread;
mod fwrite;
mod fgets;
mod fputs;
mod fgetc;
mod fputc;
mod getc;
mod getchar;
mod getc_unlocked;
mod getchar_unlocked;
mod putc;
mod putchar;
mod putc_unlocked;
mod putchar_unlocked;
mod gets;
mod puts;
mod getw;
mod putw;
mod ungetc;

pub use fread::*;
pub use fwrite::*;
pub use fgets::*;
pub use fputs::*;
pub use fgetc::*;
pub use fputc::*;
pub use getc::*;
pub use getchar::*;
pub use getc_unlocked::*;
pub use getchar_unlocked::*;
pub use putc::*;
pub use putchar::*;
pub use putc_unlocked::*;
pub use putchar_unlocked::*;
pub use gets::*;
pub use puts::*;
pub use getw::*;
pub use putw::*;
pub use ungetc::*;

// ============================================================
// 宽字符 I/O
// ============================================================
mod fgetwc;
mod fgetws;
mod fputwc;
mod fputws;
mod getwc;
mod getwchar;
mod putwc;
mod putwchar;
mod ungetwc;
mod fwide;

pub use fgetwc::*;
pub use fgetws::*;
pub use fputwc::*;
pub use fputws::*;
pub use getwc::*;
pub use getwchar::*;
pub use putwc::*;
pub use putwchar::*;
pub use ungetwc::*;
pub use fwide::*;

// ============================================================
// 流状态/缓冲操作
// ============================================================
mod fflush;
mod feof;
mod ferror;
mod clearerr;
mod fileno;
mod fseek;
mod ftell;
mod rewind;
mod fgetpos;
mod fsetpos;
mod setbuf;
mod setbuffer;
mod setlinebuf;
mod setvbuf;

pub use fflush::*;
pub use feof::*;
pub use ferror::*;
pub use clearerr::*;
pub use fileno::*;
pub use fseek::*;
pub use ftell::*;
pub use rewind::*;
pub use fgetpos::*;
pub use fsetpos::*;
pub use setbuf::*;
pub use setbuffer::*;
pub use setlinebuf::*;
pub use setvbuf::*;

// ============================================================
// 格式化输出 (varargs 包装)
// ============================================================
mod printf;
mod fprintf;
mod sprintf;
mod dprintf;
mod asprintf;
mod vprintf;
mod vsprintf;
mod vdprintf;
mod vasprintf;

pub use printf::*;
pub use fprintf::*;
pub use sprintf::*;
pub use dprintf::*;
pub use asprintf::*;
pub use vprintf::*;
pub use vsprintf::*;
pub use vdprintf::*;
pub use vasprintf::*;

// ============================================================
// 格式化输入 (varargs 包装)
// ============================================================
mod scanf;
mod fscanf;
mod sscanf;
mod vscanf;
mod vsscanf;

pub use scanf::*;
pub use fscanf::*;
pub use sscanf::*;
pub use vscanf::*;
pub use vsscanf::*;

// ============================================================
// 宽字符格式化
// ============================================================
mod wprintf;
mod fwprintf;
mod swprintf;
mod vwprintf;
mod vswprintf;
mod wscanf;
mod fwscanf;
mod swscanf;
mod vwscanf;
mod vswscanf;

pub use wprintf::*;
pub use fwprintf::*;
pub use swprintf::*;
pub use vwprintf::*;
pub use vswprintf::*;
pub use wscanf::*;
pub use fwscanf::*;
pub use swscanf::*;
pub use vwscanf::*;
pub use vswscanf::*;

// ============================================================
// 高级流操作
// ============================================================
mod fmemopen;
mod open_memstream;
mod open_wmemstream;
mod popen;
mod pclose;
mod fgetln;
mod getdelim;
mod getline;
mod perror;
mod flockfile;

pub use fmemopen::*;
pub use open_memstream::*;
pub use open_wmemstream::*;
pub use popen::*;
pub use pclose::*;
pub use fgetln::*;
pub use getdelim::*;
pub use getline::*;
pub use perror::*;
pub use flockfile::*;

// ============================================================
// 文件系统操作
// ============================================================
mod remove;
mod rename;
mod tmpfile;
mod tmpnam;
mod tempnam;

pub use remove::*;
pub use rename::*;
pub use tmpfile::*;
pub use tmpnam::*;
pub use tempnam::*;

// ============================================================
// 标准流
// ============================================================
mod stdin;
mod stdout;
mod stderr;

pub use stdin::*;
pub use stdout::*;
pub use stderr::*;

// ============================================================
// 扩展接口
// ============================================================
mod ext;
mod ext2;

pub use ext::*;
pub use ext2::*;

// ============================================================
// 打开文件列表
// ============================================================
mod ofl;
mod ofl_add;

pub(crate) use ofl::*;
pub(crate) use ofl_add::*;