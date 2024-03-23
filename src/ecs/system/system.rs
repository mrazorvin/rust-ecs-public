use crate::ecs::{
    ecs_mode,
    world::{self, World},
};
use std::marker::PhantomData;

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Stage {
    Instantination,
    Initialization,
    Execution,
}

pub mod sys_mode {
    pub enum Undefined {}
    pub enum Configuration {}
    pub enum Execution {}
}

pub struct State<EcsMode = ecs_mode::Unknown, SystemMode = sys_mode::Undefined> {
    stage: Stage,
    name: &'static str,
    world: *mut world::State<EcsMode>,
    state: Option<Box<dyn std::any::Any>>,
    sys_mod: PhantomData<SystemMode>,
}

pub fn as_sys<OutEcsMode, OutSysMode, InEcsMode, InSysMode>(
    system: &mut State<InEcsMode, InSysMode>,
) -> &mut State<OutEcsMode, OutSysMode> {
    unsafe { std::mem::transmute(system) }
}

impl<T> State<T, sys_mode::Undefined> {
    pub fn set_world(&mut self, world: *mut world::State<T>) {
        self.world = world;
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }

    pub fn set_name(&mut self, name: &'static str) {
        self.name = name;
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl State<ecs_mode::Exclusive, sys_mode::Configuration> {
    // it overall safe to create reference to world because of exclusive access
    // just don't forget about aliasing
    pub unsafe fn world(&self) -> &mut world::State<ecs_mode::Exclusive> {
        unsafe { &mut *self.world }
    }

    pub fn world_ptr(&self) -> *mut world::State<ecs_mode::Exclusive> {
        self.world
    }
}

impl<EcsMod, SysMod> State<EcsMod, SysMod> {
    pub fn new(world: *mut world::State<EcsMod>, state: Option<Box<dyn std::any::Any>>) -> Self {
        Self {
            stage: Stage::Instantination,
            name: Default::default(),
            sys_mod: PhantomData {},
            world,
            state,
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn state<'cb, 'this, T: 'static + std::any::Any>(
        &'this mut self,
        default: &'cb impl Fn() -> T,
    ) -> Result<&'this mut T, &'static str> {
        match self.state {
            Some(_) => {}
            None => {
                self.state = Some(Box::new(default()));
            }
        }

        return (&mut *self.state.as_mut().unwrap())
            .downcast_mut::<T>()
            .ok_or("system# can't downcast state, this could happened becauseyou set invalid state or because you call state more than once");
    }
}

pub fn type_name_of<T>(_: T) -> &'static str {
    let name = std::any::type_name::<T>();
    &name[5..name.len() - 3]
}

macro_rules! define {
  ($system:ident) => {
    crate::ecs::system::define!($system,);
  };
  ($system:ident, $($dep_type:ident![$type:ident]),*) => {
    crate::ecs::system::define!($system, $($dep_type![$type],)* {});
  };
  ($system:ident, $($dep_type:ident![$type:ident],)*$({$($init:tt)*})?) => {


    if $system.stage() == crate::ecs::system::Stage::Instantination {
      crate::ecs::system::define!(@sys_name $system, 5);
      $system.set_stage(crate::ecs::system::Stage::Initialization);
    }

    let $system = crate::ecs::system::as_sys::<
        crate::ecs::ecs_mode::Exclusive,
        crate::ecs::system::sys_mode::Configuration,
        _, _
    >($system);

    $(#[allow(unused_mut, non_snake_case)] let mut $type: View<_, $dep_type> = unsafe { View::new($type::fetch($system)?) };)*

    if $system.stage() == crate::ecs::system::Stage::Initialization {
      $($($init)*)?
      return crate::ecs::system::INIT;
    }

    #[allow(unused_macros)]
    macro_rules! query {
      ($$($fields:tt)*) => {
        crate::ecs::query::query_ctx!($system, $$($fields)*)
      }
    }

    #[allow(unused_macros)]
    macro_rules! query_try {
      ($$($fields:tt)*) => {{
        let _: Option<()> = try {
          crate::ecs::query::query_ctx!($system, $$($fields)*)
        };
      }}
    }

    #[allow(unused_variables)]
    let $system = crate::ecs::system::as_sys::<
        crate::ecs::ecs_mode::Exclusive,
        crate::ecs::system::sys_mode::Execution,
    _, _>($system);
  };

  (@sys_name $var:ident, $prefix_len:expr) => {
    fn f() {}
    $var.set_name(crate::ecs::system::type_name_of(f));
  };
}

#[derive(Debug, PartialEq)]
pub enum SystemResult {
    Initialized,
    Completed,
    Stop,
}

pub type SysFn = fn(&mut State<ecs_mode::Unknown, sys_mode::Undefined>) -> Return;
pub type Return = std::result::Result<SystemResult, Box<dyn std::error::Error>>;
pub const OK: std::result::Result<SystemResult, Box<dyn std::error::Error>> =
    Ok(SystemResult::Completed);
pub const STOP: std::result::Result<SystemResult, Box<dyn std::error::Error>> =
    Ok(SystemResult::Stop);
pub const INIT: std::result::Result<SystemResult, Box<dyn std::error::Error>> =
    Ok(SystemResult::Initialized);

#[test]
pub fn system_macro_test() {
    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]

    use std::marker::PhantomData;

    // #region ### Test - Unit example component
    impl Unit {
        fn fetch(
            _: &mut State<ecs_mode::Exclusive, sys_mode::Configuration>,
        ) -> Result<Unit, &str> {
            Ok(Unit { value: 0 })
        }
    }

    struct Unit {
        value: u32,
    }
    // #endregion

    // #region ### Test - Position example component

    struct Position {
        value: u32,
    }

    impl Position {
        fn fetch(
            _: &mut State<ecs_mode::Exclusive, sys_mode::Configuration>,
        ) -> Result<Position, &str> {
            Ok(Position { value: 10 })
        }
    }
    // #endregion

    #[derive(Default)]
    struct View<T, U> {
        value: T,
        _kind: PhantomData<U>,
    }

    impl<T, U> View<T, U> {
        unsafe fn new(value: T) -> View<T, U> {
            View { value, _kind: PhantomData {} }
        }
    }

    struct read {}
    struct write {}

    fn system_features_fn(sys: &mut State) -> Return {
        define!(sys, read![Position], write![Unit]);

        assert_eq!(Position.value.value, 0);
        assert_eq!(Unit.value.value, 10);

        Position.value = Position { value: 20 };
        Unit.value = Unit { value: 10 };

        let _: View<Position, read> = Position;
        let _: View<Unit, write> = Unit;

        return OK;
    }
}

#[allow(unused)]
pub(crate) use define;
