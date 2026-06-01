//! 所有集成测试子模块

mod drand48_test;
mod lcong48_test;
mod lrand48_test;
mod mrand48_test;
mod random_test;
mod rand_r_test;
mod rand_test;
mod seed48_test;
mod srand48_test;

pub use rusl::api::prng::*;