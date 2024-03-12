#![allow(non_snake_case)]

use crate::ecs::{components::Components, entity::entity_id};

// ### Testing structures
#[derive(Debug, Default)]
struct Position {
    x: u32,
    y: u32,
}

#[derive(Debug, Default)]
struct Hero {
    fame: u64,
}
// #endregion

fn create_data(
    archs_amount: usize,
    components_per_arch: usize,
) -> (Box<Components<Position>>, Box<Components<Hero>>) {
    // #region ### Validation

    assert!(archs_amount >= 1);
    assert!(components_per_arch < u16::MAX as usize);
    // #endregion

    // #region ### Components store

    let mut Position: Components<Position> = Components::default();
    let mut Hero: Components<Hero> = Components::default();
    // #endregion

    for arch_id in 1..=archs_amount {
        for entity_id in 1..=components_per_arch {
            Position.set(arch_id, entity_id, Position { x: entity_id as u32, y: entity_id as u32 });
            Hero.set(arch_id, entity_id, Hero { fame: u64::MAX });
        }
    }

    (Box::new(Position), Box::new(Hero))
}

#[test]
fn query_create_data() {
    use std::num::NonZeroU16;

    // ### Test - initial state
    let (_, mut Hero) = create_data(1, 5);
    assert_eq!(unsafe { Hero.chunks.as_slice().len() }, 1);
    assert_eq!(Hero.len(), 5);

    // ### Tets - update existed
    Hero.set(1, 5, Hero { fame: u64::MAX });
    assert_eq!(unsafe { Hero.chunks.as_slice().len() }, 1);
    assert_eq!(Hero.len(), 5);

    // ### Test - new component for existed arch
    Hero.set(1, 6, Hero { fame: u64::MAX });
    assert_eq!(unsafe { Hero.chunks.as_slice().len() }, 1);
    assert_eq!(Hero.len(), 6);

    // ### Test - new component for new arch
    Hero.set(2, 1, Hero { fame: u64::MAX });
    assert_eq!(unsafe { Hero.chunks.as_slice().len() }, 2);
    assert_eq!(Hero.len(), 7);

    // ### Test - sorting
    Hero.set(10, 1, Hero { fame: u64::MAX });
    assert_eq!(unsafe { Hero.chunks.as_slice().len() }, 3);
    assert_eq!(Hero.len(), 8);

    Hero.set(5, 1, Hero { fame: u64::MAX });
    assert_eq!(unsafe { Hero.chunks.as_slice().len() }, 4);
    assert_eq!(Hero.len(), 8);

    assert_eq!(
        vec![1, 2, 5, 10],
        unsafe { Hero.chunks.as_slice().iter().map(|(id, _)| *id) }.collect::<Vec<_>>(),
    );
    // #endregion
}

#[derive(Default)]
struct Stats {}

#[test]
fn query_iterate_entities() {
    use crate::ecs::view::*;

    let (Position, Hero) = create_data(10, 10);

    let Hero: View<Components<Hero>, read> = unsafe { View::new(Box::into_raw(Hero)) };
    let Position: View<Components<Position>, write> = unsafe { View::new(Box::into_raw(Position)) };

    let _query_params = ();

    // #region ### Test - simple iterate
    let mut result = 0;
    crate::ecs::query::query_ctx!(_query_params, Position[_], Hero[_], |_| {
        result += 1;
    });
    assert_eq!(result, 100);
    // #endregion

    let mut Stats = Components::default();
    for i in 0..10 {
        Stats.set(150, 150 + i, Stats {});
    }
    let Stats: View<Components<Stats>, read> = unsafe { View::new(Box::into_raw(Box::new(Stats))) };

    // #region ### test - iterate over less than 64 items
    let mut result = 0;
    crate::ecs::query::query_ctx!(query_params, Stats[_], |_| {
        result += 1;
    });
    assert_eq!(result, 10);
    // #endregion

    // #region ### test - iterate over different sizes views
    let mut result = 0;
    crate::ecs::query::query_ctx!(query_params, Position[_], Hero[_], Stats[_], |_| {
        result += 1;
    });
    assert_eq!(result, 0);
    // #endregion

    // Test inserting new item during iterations (with memory reloacation) & inserting new archetype (with memory relocation)

    drop(unsafe { Box::from_raw(Hero.data_ptr()) });
    drop(unsafe { Box::from_raw(Position.data_ptr()) });
    drop(unsafe { Box::from_raw(Stats.data_ptr()) });
}

#[test]
fn new_items_while_iterations() {
    use crate::ecs::view::*;
    use std::num::NonZeroU16;

    let (Position, Hero) = create_data(10, 10);
    let Hero: View<Components<Hero>, write> = unsafe { View::new(Box::into_raw(Hero)) };
    let Position: View<Components<Position>, write> = unsafe { View::new(Box::into_raw(Position)) };
    let _query_params = ();

    // #region ### Test - Inserting new items
    let mut result = 0;
    let max_arch = 10usize;
    assert_eq!(Position.len(), 100);
    assert_eq!(Position.len(), 100);
    crate::ecs::query::query_ctx!(_query_params, Position[_], Hero[_], |_| {
        if result < max_arch {
            unsafe { Position.try_set(11 + result.pow(2), 100, Position::default()) };
            unsafe { Hero.try_set(11 + result.pow(2), 100, Hero::default()) };
        }
        result += 1;
    });
    assert_eq!(result, 100);
    assert_eq!(Position.len(), 110);
    assert_eq!(Hero.len(), 110);
    // #endregion

    let mut result = 0;
    crate::ecs::query::query_ctx!(params, Position[_], Hero[_], |_| {
        result += 1;
    });
    assert_eq!(result, 110);

    drop(unsafe { Box::from_raw(Hero.data_ptr()) });
    drop(unsafe { Box::from_raw(Position.data_ptr()) });
}
