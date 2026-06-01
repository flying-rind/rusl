//! PRNG (伪随机数生成器) — Rusl 实现 libc 的随机数生成函数。
//! 根据 rust-spec 文件自动生成接口骨架和单元测试。

#![allow(unused_imports)]

pub mod __rand48_step;
pub mod __seed48;
mod drand48;
mod lcong48;
mod lrand48;
mod mrand48;
mod rand;
mod rand_r;
mod random;
mod seed48;
mod srand48;

pub use drand48::{drand48, erand48};
pub use lcong48::lcong48;
pub use lrand48::{lrand48, nrand48};
pub use mrand48::{jrand48, mrand48};
pub use rand::{rand, srand, RAND_MAX};
pub use rand_r::rand_r;
pub use random::{initstate, random, setstate, srandom};
pub use seed48::seed48;
pub use srand48::srand48;
pub use __seed48::__seed48;
