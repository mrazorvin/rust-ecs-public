#![allow(dead_code)]
#![allow(internal_features)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![feature(negative_impls)]
#![feature(core_intrinsics)]
#![feature(macro_metavar_expr)]
#![feature(thread_local)]
#![feature(try_blocks)]

mod application;
mod client;
mod ecs;
mod game_arpg;

// shipyard = "0.6.2"
// mod perf;

#[cfg(not(target_os = "android"))]
fn main() {
    // perf::main();
    application::start_app();
}

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
#[no_mangle]
#[allow(non_snake_case)]
pub fn SDL_main() {
    android::android_log_thread::spawn_android_log_thread();
    application::start_app();
}
