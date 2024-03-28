#![allow(dead_code)]
#![allow(internal_features)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![feature(negative_impls)]
#![feature(core_intrinsics)]
#![feature(macro_metavar_expr)]
#![feature(thread_local)]
#![feature(try_blocks)]

use std::{any::TypeId, collections::HashMap, num::NonZeroU32};

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

// store all resources into RwLock<HashMap<TypeId, SlotMap<*mut ()>>> - or something like that for top level access, when access is never take RwLock for longer than insert duration, beside that make same struct for global string interner, then SlotMap could be accessed via id or string

// struct Res1 {}
// struct Res2 {}
//
// trait SystemDeps: Sized {
//     fn get(map: HashMap<TypeId, *mut ()>) -> Self {
//         unimplemented!()
//     }
// }
//
// impl<'a, A: 'static, B: 'static> SystemDeps for (&'a A, &'a B) {
//     fn get(map: HashMap<TypeId, *mut ()>) -> (&'a A, &'a B) {
//         let a = *map.get(&std::any::TypeId::of::<A>()).unwrap();
//         let b = *map.get(&std::any::TypeId::of::<B>()).unwrap();
//
//         unsafe { (&*(a as *const A), &*(b as *const B)) }
//     }
// }
//
// fn cool_fn(resource: (&Res1, &Res2), args: (usize, usize)) {}
//
// fn run<T: SystemDeps, A, R>(funcn: fn(T, A) -> R, args: A) -> R {
//     funcn(T::get(unimplemented!()), args)
// }

// fn main() {
//     run(cool_fn, (23, 23));
// }

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
#[no_mangle]
#[allow(non_snake_case)]
pub fn SDL_main() {
    android::android_log_thread::spawn_android_log_thread();
    application::start_app();
}
