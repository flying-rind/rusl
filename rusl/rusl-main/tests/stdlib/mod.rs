//! 所有集成测试子模块

mod abs_test;
mod atof_test;
mod atoi_test;
mod bsearch_test;
mod div_test;
mod ecvt_test;
mod labs_test;
mod qsort_test;
mod strtod_test;
mod strtol_test;
mod wcstod_test;
mod wcstol_test;

pub use rusl::api::stdlib::*;

// 根据 rusl feature 选择导入源
#[cfg(feature = "rusl")]
mod imports {
    pub use rusl_stdlib::*;
}
#[cfg(not(feature = "rusl"))]
mod imports {
    pub use rusl::api::stdlib::*;
}