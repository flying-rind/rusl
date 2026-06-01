//! 所有集成测试子模块

mod hsearch_test;
mod insque_test;
mod lsearch_test;
mod tdestroy_test;
mod twalk_test;

pub extern crate alloc;

// 根据 rusl feature 选择导入源
#[cfg(feature = "rusl")]
mod imports {
    pub use rusl_search::*;
}
#[cfg(not(feature = "rusl"))]
mod imports {
    pub use rusl::api::search::*;
}