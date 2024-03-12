#![allow(dead_code)]
#![allow(internal_features)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![feature(negative_impls)]
#![feature(core_intrinsics)]
#![feature(macro_metavar_expr)]
#![feature(thread_local)]
#![feature(try_blocks)]

pub mod client;
pub mod ecs;
pub mod game_wolf_survivors;

#[cfg(target_os = "android")]
pub mod android;
