//! 标准库函数 —— Rusl 实现的 libc stdlib 函数。
//! 根据 spec 文件自动生成的代码骨架。

#![allow(dead_code, unused_imports)]

mod abs;
mod atof;
mod atoi;
mod bsearch;
mod div;
mod ecvt;
mod labs;
mod qsort;
mod strtod;
mod strtol;
mod wcstod;
mod wcstol;

pub use abs::abs;
pub use atof::atof;
pub use atoi::{atoi, atol, atoll};
pub use bsearch::bsearch;
pub use div::{div, div_t, imaxdiv, imaxdiv_t, ldiv, ldiv_t, lldiv, lldiv_t};
pub use ecvt::{ecvt, fcvt, gcvt};
pub use labs::{imaxabs, labs, llabs};
pub use qsort::{qsort, qsort_r, CmpFun, CmpFunR};
pub use strtod::{strtod, strtof, strtold};
pub use strtol::{strtoimax, strtol, strtoll, strtoull, strtoul, strtoumax};
pub use wcstod::{wcstod, wcstof, wcstold};
pub use wcstol::{wcstol, wcstoll, wcstoul, wcstoull, wcstoimax, wcstoumax};