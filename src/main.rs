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

// impl def, then impl fetch for multi and single

// impl for Unpin {}
struct Collection<T> {
    values: Vec<T>,
}

enum Components {}
enum Reousrces {}
trait Resource<Kind>: Sized + 'static {
    type Target = Self;
    type Dispose = AutoDisposable;
}

enum AutoDisposable {}
enum AutoClone {}

struct Position {}
impl Resource<Components> for Position {}

// how to differ components from single resource

impl<U, T: Resource<Components, Target = U, Dispose = AutoDisposable>> Dispose for T {
    fn dispose(&self) {
        println!("CUSTOM DISPOSE");
    }
}

trait Dispose {
    fn dispose(&self) {}
}

trait Component: Dispose {}
impl<T: Dispose> Component for T {}

// trait NotAll {
//     fn dispose(&self, c: usize) {}
// }

// impl All for Vec<u32> {}

// trait B {}
// trait A {}
// impl !All for Vec<u32> {}

#[cfg(not(target_os = "android"))]
fn main() {
    let component: &dyn Component = &Position {};

    // component.

    // dispose(&vec![]);

    // perf::main();
    // application::start_app();
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
