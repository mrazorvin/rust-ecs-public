use std::sync::atomic::AtomicU64;

use crate::ecs::collections::sync_slot::SyncSlotMap;

pub struct Timer {
    func: fn(*const (), timer: Self) -> (),
    arch: *const (),
    component_id: u32,
    end: u32,
}

pub struct ComponetsTimers {
    pub timers: SyncSlotMap<Timer>,
}

impl ComponetsTimers {
    pub fn new() -> ComponetsTimers {
        ComponetsTimers { timers: SyncSlotMap::new() }
    }
}

// struct MyAmazinStruct<T> {
//     data: T,
//     payload: usize,
// }
//
// impl<T> MyAmazinStruct<T> {
//     fn log(this: *mut Self, param: u32) {
//         println!("{} - {param} {}", std::any::type_name_of_val(&this), unsafe { (*this).payload });
//     }
// }
//
// struct Data(u32);
//
// struct Callback {
//     func: fn(*mut (), param: u32) -> (),
//     this: *mut (),
//     param: u32,
// }
//
// fn main() {
//     let mut gen = MyAmazinStruct { data: String::from("Hello World"), payload: 999 };
//     let mut simpl = Data(222);
//     let vec = vec![
//         Callback {
//             func: unsafe { std::mem::transmute(MyAmazinStruct::<String>::log as *const ()) },
//             this: &mut gen as *mut _ as *mut (),
//             param: 100,
//         },
//         Callback {
//             func: {
//                 fn data(this: &Data, param: u32) {
//                     println!("Data: {} - {}", this.0, param);
//                 }
//                 unsafe { std::mem::transmute(data as *const ()) }
//             },
//             param: 200,
//             this: &mut simpl as *mut _ as *mut (),
//         },
//     ];
//
//     for cb in vec {
//         (cb.func)(cb.this, cb.param);
//     }
// }
