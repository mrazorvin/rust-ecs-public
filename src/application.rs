use std::{cell::Cell, time::Instant};

use crate::{ecs::prelude::*, game_arpg};

pub fn start_app() {
    let tests = Box::leak(Box::new(Vec::new()));

    tests.push(("ecs", crate::ecs::integration_tests()));
    tests.push(("client", crate::client::integration_tests()));

    let world = world::World::new();

    let module_id = Cell::new(0);
    let test_id = Cell::new(0);
    let test_executor = |world: &mut world::World| {
        let module = tests.get(module_id.get());
        if let Some((_, tests)) = module {
            if let Some(test_func) = tests.get(test_id.get()) {
                world.add_system_with_state(
                    test_executor,
                    Some(Box::new(*test_func)),
                    Schedule::Update,
                );
                test_id.set(test_id.get() + 1);
            } else {
                module_id.set(module_id.get() + 1);
            }
        } else {
            game_arpg::create_game(world).unwrap();
        }
    };

    crate::client::render_loop(world, &test_executor, &frame_dispose::disposer).unwrap()
}

fn test_executor(sys: &mut world::System) -> system::Return {
    let func = sys.state(&|| -> system::Func<world::State> { test_placeholder_func })?;
    let now = Instant::now();
    let result = func(sys);
    let elapsed = now.elapsed().as_micros();
    if elapsed > 1000 {
        println!("OK {}ms - {}", now.elapsed().as_millis(), sys.name());
    } else {
        println!("OK {}micro - {}", now.elapsed().as_micros(), sys.name());
    }
    result
}

fn test_placeholder_func(_: &mut world::System) -> system::Return {
    panic!("test call placeholder instead of real function");
}
