use crate::ecs::prelude::*;

pub fn create_game(
    world: &mut crate::ecs::world::World<ecs_mode::Exclusive>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("----- Cycle -----");

    world.add_system(init, Schedule::Update);
    world.add_system(setup, Schedule::Update);

    Ok(())
}

pub fn init(sys: &mut system::State) -> system::Return {
    system::define!(sys, {
        println!("ARPG(init): Initialization");
        sys.add_system(sys1, Schedule::Update);
        sys.add_system(sys2, Schedule::Update);
    });

    println!("ARPG(init): Execution");

    system::OK
}

pub fn setup(sys: &mut system::State) -> system::Return {
    system::define!(sys, {});

    println!("ARPG(setup): Execution");

    system::OK
}

pub fn sys1(sys: &mut system::State) -> system::Return {
    system::define!(sys);

    println!("ARPG(sys1): Execution");

    system::OK
}

pub fn sys2(sys: &mut system::State) -> system::Return {
    system::define!(sys);

    println!("ARPG(sys2): Execution");

    system::OK
}
