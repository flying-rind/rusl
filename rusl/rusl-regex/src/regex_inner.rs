//! REGEX 模块 — Rusl 实现的 POSIX 正则表达式和通配符匹配函数。
//!
//! 根据 spec 文件自动生成，函数体为 `todo!()` 占位。
//!
//! # 模块结构
//!
//! ## 对外公开接口
//!
//! - `fnmatch`（Shell 通配符匹配） + `FNM_*` 常量
//! - `glob` / `globfree`（文件系统路径展开） + `GLOB_*` 常量 + `glob_t`
//! - `regcomp` / `regfree`（正则表达式编译/释放） + `REG_*` 常量 + `regex_t` / `regmatch_t`
//! - `regerror`（错误消息转换）
//! - `regexec`（正则表达式匹配） + `REG_NOTBOL` / `REG_NOTEOL`
//!
//! ## 内部模块（不对外导出）
//!
//! - `tre`：TRE 引擎内部类型、常量和 TNFA 数据结构
//! - `tre_mem`：Bump-pointer 内存分配器
//! - `regcomp_ast`：AST 类型定义和构造函数
//! - `regcomp_parse`：正则表达式解析器
//! - `regcomp_transform`：Tag 注入和 AST 迭代展开
//! - `regcomp_nfl`：NFL 计算和 TNFA 构建
//! - `regexec_parallel`：并行 NFA 模拟匹配器
//! - `regexec_backtrack`：深度优先回溯匹配器

#![allow(dead_code, unused_imports)]

// ---- 内部基础模块（不对外导出） ----

mod tre;
mod tre_mem;

// ---- regcomp 内部子模块（不对外导出） ----

mod regcomp_ast;
mod regcomp_parse;
mod regcomp_transform;
mod regcomp_nfl;

// ---- regexec 内部子模块（不对外导出） ----

mod regexec_parallel;
mod regexec_backtrack;

// ---- 公开模块 ----

mod fnmatch;
mod glob;
mod regcomp;
mod regerror;
mod regexec;

// ---- 公开 API 重导出 ----

// fnmatch（Shell 通配符匹配）
pub use fnmatch::{
    fnmatch,
    FNM_CASEFOLD,
    FNM_LEADING_DIR,
    FNM_NOESCAPE,
    FNM_NOMATCH,
    FNM_NOSYS,
    FNM_PATHNAME,
    FNM_PERIOD,
};

// glob（文件系统路径展开）
pub use glob::{
    glob,
    globfree,
    glob_t,
    GLOB_ABORTED,
    GLOB_APPEND,
    GLOB_DOOFFS,
    GLOB_ERR,
    GLOB_MARK,
    GLOB_NOCHECK,
    GLOB_NOESCAPE,
    GLOB_NOMATCH,
    GLOB_NOSORT,
    GLOB_NOSPACE,
    GLOB_PERIOD,
    GLOB_TILDE,
    GLOB_TILDE_CHECK,
};

// regcomp（正则表达式编译）
pub use regcomp::{
    regcomp,
    regfree,
    regmatch_t,
    regoff_t,
    regex_t,
    REG_BADBR,
    REG_BADPAT,
    REG_BADRPT,
    REG_EBRACE,
    REG_EBRACK,
    REG_ECOLLATE,
    REG_ECTYPE,
    REG_EESCAPE,
    REG_ENOSYS,
    REG_EPAREN,
    REG_ERANGE,
    REG_ESPACE,
    REG_ESUBREG,
    REG_EXTENDED,
    REG_ICASE,
    REG_NEWLINE,
    REG_NOMATCH,
    REG_NOSUB,
    REG_OK,
};

// regerror（错误消息转换）
pub use regerror::regerror;

// regexec（正则表达式匹配）
pub use regexec::{
    regexec,
    REG_NOTBOL,
    REG_NOTEOL,
};
