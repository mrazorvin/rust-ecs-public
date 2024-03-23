#![allow(non_snake_case)]
use crate::ecs::{components::ComponnetsResource, prelude::*};

// ### Position components
impl ComponnetsResource for Position {}
struct Position {}

// ### Resource
impl UniqueResource for Input {}
struct Input {
    x: u32,
    y: u32,
}

impl Input {
    pub fn print_pos(&self) {
        println!("x: {}, y: {}", self.y, self.x);
    }
}

#[test]
fn world_test() {
    let mut world = world::World::new();

    world.add_unique(Input { x: 10, y: 20 }).unwrap();
    world.add_system(simple_system, Schedule::Update);

    world.execute();
    world.execute();
}

fn simple_system(sys: &mut system::State) -> system::Return {
    system::define!(sys, write![Position], write![Input]);

    Position.set(10, 10, Position {});
    Input.print_pos();

    let mut resulted_entity_id = 0;
    query!(Position[_], |entity_id| {
        resulted_entity_id = entity_id;
    });
    assert_eq!(resulted_entity_id, 10);

    return system::OK;
}
