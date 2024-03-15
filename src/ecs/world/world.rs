use bumpalo::boxed::Box as BumpBox;
use hashlink::LinkedHashMap;
use nohash_hasher::NoHashHasher;
use std::{
    any::{self, Any, TypeId},
    collections::HashMap,
    hash::{BuildHasherDefault, Hash},
    ops::{Deref, DerefMut},
};

use crate::ecs::system::{self, Stage, SystemResult, OK};

pub type System = system::State<State>;

//  Basic implementation without priority consist of folowing steps:
//  - Single vec for stage 1 & stage 2
//  - 2 separate vecs for IO & Single thread + LUA based dnamic immutable
//  - Single vec for stage 4
//
//
// 4 scheduling targets
// - Startup
//   - Single sequence + Priorities (i.e you can add system early)
//   - Mutable access: All
//
//  Both Startup & Workload could be chained with following logic
//  - Store in Resource & Components last system that used in startup stage (we still need to do this because of chaining)
//
//   The biggest priority of startup is to unlock as many system as possible, that important especially for system that could be runned on main thread, they have biggest prioority
//
//   - Then since we know that main thread access transformation and oher things we could give it some tip to access this resources (maybe based on Rendering dependecies), in any cases optimizing main thread have the biggest priority, so we could give *tip on

//   - Second optimization could be done for LUA engines to run as many system that related to current engine as much

//  - Teardown also may priorty for main thread mutable world access & systems that works with already loaded components into memory

//  Is it worth to optimize not second mian thread ? maybe something based on sequnces
//  when sequnce access same aresource.
//
//  In all opptimization cases if there nothing to do, we should allow execute tiiped functions
//
// - Workload:
//   + Multiple independent seqeunces based on defined function
//     - In every new system reset system chain to zero
//   + Mutable access: All
//
// IMPORTANT: This stages can't be chained safely because we don't know which system will be accesed by LUA engine, us also kind unsafe because we could block with lua some mutabel componenets which also could block some improat sequnce for no good reason
//
// - IO & Rendering & immutable no ordering systems:
//   Single thread IO Systems & Rendering
//      + Priorities (aka Ordering because of single thread) -> every system will be executed on main thread and must have explicit order in which
//      - Mutable access: Resources
//      - Sequnce
//
//   Immutable read only scripring system:
//      - Sequnce / Ordering / Priorities
//      - Mutable access: No
//      - Readonly access: No IO Resources
//      + You can add dynamically any system that follow rules above to this stage
//
//  IMPORTANT: This stages can't be chained at all, because LUA systemss could update things via Patch API's
//
// - Tearwdown systems (
//   + Mutable access: All
//   - Sequnce / Ordering / Priorities
//   + We could add any system to this stage at any point, even with fully mutable access to world
//
//

// we can't use allocations for this struc, otherwise change `set_len` to `clear` in
pub struct ScheduleItem {
    func: system::Func<State>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Schedule {
    Startup,
    Update,
    Render,
    Scripting,
    Teardown,
}

pub struct SchedulerStage {
    items: Vec<ScheduleItem>,
}

impl SchedulerStage {
    pub fn add_system(&mut self, func: system::Func<State>) {
        self.items.push(ScheduleItem { func })
    }
}

pub struct SchedulerStages {
    startup: SchedulerStage,
    update: SchedulerStage,
    render: SchedulerStage,
    scripting: SchedulerStage,
    teardown: SchedulerStage,
}

impl SchedulerStages {
    pub fn new() -> Self {
        Self {
            startup: SchedulerStage { items: Vec::new() },
            update: SchedulerStage { items: Vec::new() },
            render: SchedulerStage { items: Vec::new() },
            scripting: SchedulerStage { items: Vec::new() },
            teardown: SchedulerStage { items: Vec::new() },
        }
    }

    pub fn schedule_system(_world: *mut World, _system: system::Func<State>) {}

    pub fn reset(&mut self) {
        // SAFETY: we don't need to drop function pointers, and 0 is always less than vec capacity
        unsafe {
            self.startup.items.set_len(0);
            self.update.items.set_len(0);
            self.render.items.set_len(0);
            self.scripting.items.set_len(0);
            self.teardown.items.set_len(0);
        };
    }
}

pub struct Scheduler {
    stages: SchedulerStages,
}

impl Scheduler {
    pub fn reset(&mut self) {
        self.stages.reset();
    }

    fn schedule_system(world: *mut State, system_fn: system::Func<State>, schedule: Schedule) {
        // SAFETY: at any point of time should exist only single reference to world
        //         in this cases lifetime is ended after end of the function
        //         so if outher functions doens't has mut refrence to world it's safe to call
        let State { ref mut scheduler, .. } = unsafe { &mut *world };

        match schedule {
            Schedule::Startup => todo!(),
            Schedule::Update => {
                scheduler.stages.update.add_system(system_fn);
            }
            Schedule::Render => todo!(),
            Schedule::Scripting => todo!(),
            Schedule::Teardown => todo!(),
        }
    }
}

pub struct State {
    pub systems: HashMap<
        // usize is just a pointer to memory, it could be pointer to a state if system spawned with state or directly to function pointer if there no state
        usize,
        system::State<State>,
        BuildHasherDefault<NoHashHasher<u64>>,
    >,
    pub scheduler: Scheduler,
    pub resources: HashMap<
        any::TypeId,
        (*mut u8, BumpBox<'static, dyn any::Any>),
        // SAFETY: 24.2.2024 - TypeId use u64 in hash function, if they switch to type that no suppoerted by `NoHashHasher`` application will panic at the start
        BuildHasherDefault<NoHashHasher<u64>>,
    >,
    pub resources_bump: bumpalo::Bump,
}

impl Default for State {
    fn default() -> Self {
        Self {
            resources: Default::default(),
            resources_bump: Default::default(),
            scheduler: Scheduler { stages: SchedulerStages::new() },
            systems: Default::default(),
        }
    }
}

impl State {
    fn schedule_system(
        world: *mut State,
        system_fn: system::Func<State>,
        state: Option<Box<dyn Any>>,
        schedule: Schedule,
    ) {
        // SAFETY: at any point of time should exist only single reference to world
        //         in this cases lifetime of &mut world is ended after assigment
        //         so if outher functions doens't has mut refrence to world it's safe to call
        let state = unsafe { &mut *world }
            .systems
            .entry(system_fn as usize)
            .or_insert_with(|| system::State::new(world, state));

        match system_fn(state) {
            Err(error) => panic!(
                "World.add_sytem# system {} initialization failed with error {}",
                state.name(),
                error
            ),
            Ok(SystemResult::Stop) => panic!(
                "World.add_sytem# system {} initialization failed, because system returns stop",
                state.name()
            ),
            Ok(SystemResult::Completed) => {
                panic!(
                    "World.add_sytem# system {} initialization failed, system::define! not used",
                    state.name()
                )
            }
            Ok(SystemResult::Initialized) => {
                Scheduler::schedule_system(world, system_fn, schedule);
            }
        }
    }

    pub fn set_resource<T: Resource>(&mut self, data: T) -> (*mut T, Option<T>) {
        let existed_resource = self
            .resources
            .get(&TypeId::of::<T>())
            .map(|(ptr, _)| unsafe { std::mem::transmute::<*mut u8, *mut T>(*ptr) });

        match existed_resource {
            Some(existed_ptr) => (existed_ptr, Some(unsafe { existed_ptr.replace(data) })),
            None => {
                let ptr = BumpBox::into_raw(BumpBox::new_in(data, &self.resources_bump));
                let insertion_result = self
                    .resources
                    .insert(TypeId::of::<T>(), (ptr as *mut u8, unsafe { BumpBox::from_raw(ptr) }));
                match insertion_result {
                    Some(_) => panic!("State.set_resource# something went wrong, type {} can't exists in world at this point, probably you call State.set_resource in multiple threads", any::type_name::<T>()),
                    None => (ptr, None),
                }
            }
        }
    }

    pub fn add_resource<T: Resource>(&mut self, data: T) -> Result<*mut T, String> {
        match self.set_resource(data) {
            (_, Some(_)) => Err(format!("{} already exists in world", any::type_name::<T>())),
            (ptr, None) => Ok(ptr),
        }
    }

    pub fn add_unique<T: UniqueResource>(&mut self, data: T) -> Result<*mut Unique<T>, String> {
        match self.set_resource(Unique { data }) {
            (_, Some(_)) => {
                Err(format!("Unique {} already exists in world", any::type_name::<T>()))
            }
            (ptr, None) => Ok(ptr),
        }
    }

    pub fn set_unique<T: UniqueResource>(
        &mut self,
        data: T,
    ) -> (*mut Unique<T>, Option<Unique<T>>) {
        self.set_resource(Unique { data })
    }
}

pub trait Resource: 'static {}
impl<T: UniqueResource> Resource for Unique<T> {}

#[repr(transparent)]
pub struct Unique<T> {
    data: T,
}

impl<T> Deref for Unique<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Unique<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

pub trait UniqueResource: 'static + Sized {
    fn fetch(
        system: &mut system::State<State, system::stage_kind::Initilization>,
    ) -> Result<*mut Unique<Self>, String> {
        let components_ptr = unsafe { &*system.world() }
            .resources
            .get(&TypeId::of::<Unique<Self>>())
            .map(|(ptr, _)| unsafe { std::mem::transmute::<*mut u8, *mut Unique<Self>>(*ptr) });

        match system.stage() {
            Stage::Instantination => Err(format!(
                "Instantination stage can't be used to fetch {}",
                any::type_name::<Unique<Self>>()
            )),
            Stage::Initialization => match components_ptr {
                Some(ptr) => Ok(ptr),
                None => panic!(
                    "Unique {} resources can't be added into world dybnamically",
                    any::type_name::<Unique<Self>>()
                ),
            },
            Stage::Execution => components_ptr
                .ok_or(format!("{} non exists in world", any::type_name::<Unique<Self>>())),
        }
    }
}

impl system::State<super::State, system::stage_kind::Initilization> {
    pub fn add_system(&mut self, system_fn: system::Func<State>, schedule: Schedule) {
        State::schedule_system(self.world(), system_fn, None, schedule);
    }
}

pub struct World {
    pub state: *mut State,
}

impl !Send for World {}
impl !Sync for World {}

impl World {
    pub fn new() -> World {
        World { state: Box::into_raw(Box::new(super::State::default())) }
    }

    pub fn add_system_with_state(
        &mut self,
        system_fn: system::Func<State>,
        system_state: Option<Box<dyn Any>>,
        schedule: Schedule,
    ) {
        State::schedule_system(self.state, system_fn, system_state, schedule);
    }

    pub fn add_system(&mut self, system_fn: system::Func<State>, schedule: Schedule) {
        self.add_system_with_state(system_fn, None, schedule);
    }

    pub fn execute(&mut self) {
        // #region ### World -> Executre -> System call
        for system_id in 0..self.scheduler.stages.update.items.len() {
            let system_fn = self.scheduler.stages.update.items[system_id].func;
            let key = self.scheduler.stages.update.items[system_id].func as usize;
            let system = self.systems.get_mut(&key).unwrap();
            system.set_stage(Stage::Execution);
            match system_fn(system) {
                Err(error) => panic!("System {}() failed with error# `{}`", system.name(), error),
                Ok(SystemResult::Initialized) => {
                    panic!("System {}, return `Initialized` in Execution state", system.name())
                }
                Ok(SystemResult::Completed) => {}
                Ok(SystemResult::Stop) => {
                    // how to handle this ? do we need only set some atomic value
                    // but adding new system is required to iterate over entire list
                    // or we need to remove this atomically
                }
            }
            system.set_stage(Stage::Initialization);
        }
        // #endregion
        self.scheduler.reset();
    }
}

// #region ### World -> Deref & DerefMut
impl Deref for World {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.state }
    }
}

impl DerefMut for World {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.state }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.state) });
    }
}
// #endregion

#[test]
fn test_insetion_order_map() {
    let mut hashmap = HashMap::new();
    let mut hashlink = LinkedHashMap::new();
    for i in (-5..10).rev() {
        hashmap.insert(i, i);
        hashlink.insert(i, i);
    }

    let vec = (-5..10).rev().collect::<Vec<_>>();
    assert_ne!(hashmap.iter().map(|(_, v)| { *v }).collect::<Vec<_>>(), vec);
    assert_eq!(hashlink.iter().map(|(_, v)| { *v }).collect::<Vec<_>>(), vec);
}
