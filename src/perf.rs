use crate::ecs;
use bumpalo::{boxed::Box as BumBox, Bump};
use nanorand::{Rng, WyRand};
use std::{
    marker::PhantomData,
    num::NonZeroU16,
    ops::Index,
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Default, Debug)]
struct Stats {
    hp: i32,
    damage: i32,
}

#[derive(Default, Debug)]
struct Hero {
    name: i128,
}

#[derive(Default, Debug)]
struct Position {
    x: i32,
    y: i32,
}

pub fn main() {
    let _ = iterate_entities_sparse();
    let _ = iterate_entities_sync_sparse();
    let _ = iter_shipayrd();
}

// average iteration for read only version is 140ms
// average iteration for write version is 170ms
fn iterate_entities_sync_sparse() -> i32 {
    let mut Position = Box::new(ecs::components::Components::default());
    let mut Stats = Box::new(ecs::components::Components::default());
    let mut Hero = Box::new(ecs::components::Components::default());

    let now = Instant::now();
    for arch_id in 0..16 {
        for entity_id in 0..4096 {
            Position.set(arch_id, entity_id, Position { x: 10 as i32, y: 10 as i32 });
            Stats.set(arch_id, entity_id, Stats { hp: 10 as i32, damage: 10 as i32 });
            Hero.set(arch_id, entity_id, Hero { name: 123123123123123123123123123 });
        }
    }
    let data_insertion_time = now.elapsed().as_micros();

    println!("");
    println!("Sync archetypes entities  : {}", Position.len());
    println!("Sync archetypes inserting : {}micro", data_insertion_time);

    let Position = unsafe { ecs::prelude::View::new_read(Box::into_raw(Position)) };
    let Stats = unsafe { ecs::prelude::View::new_read(Box::into_raw(Stats)) };
    let Hero = unsafe { ecs::prelude::View::new_read(Box::into_raw(Hero)) };

    sleep(Duration::from_millis(2010));

    let now = Instant::now();
    let qp = ();
    let mut result1 = 0;
    let mut result2 = 0;
    ecs::prelude::query_ctx!(qp, Position[&pos], Stats[&stats], Hero[&hero], |_| {
        result1 += 1;
        result2 += (pos.x + stats.damage) as i128 + hero.name;
    });
    let time = now.elapsed().as_micros();
    println!("Sync archetypes result   : {:?}", result2);
    println!("Sync archetypes entities : {:?}", result1);
    println!("Sync archetypes time     : {}micro", time);

    0
}

// average iteration times around 170ms
fn iterate_entities_sparse() -> i32 {
    use ecs::collections::sparse_array::{SparseArray, SparseSlot};

    #[derive(Default)]
    struct Collection<T> {
        _marker: PhantomData<T>,
        value: u32,
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    #[repr(C)]
    struct EntityId {
        id: NonZeroU16,
        arch: NonZeroU16,
        arch_offset: u16,
    }

    type BitsChunk = u64;
    const BITS_SIZE: u16 = 64;

    struct Components<Data: Default> {
        len: usize,
        chunks: SparseArray<ArchChunk<Data>>,
        archs: Vec<u64>,
        bump: Bump,
    }

    impl<T: Default> Components<T> {
        fn push(&mut self, data: T, id: EntityId) {
            self.len += 1;
            let mut chunk = self.chunks.get_mut_(id.arch.get());
            if chunk.arch != id.arch {
                let sparse = BumBox::into_raw(BumBox::new_in(SparseArray::default(), &self.bump));
                self.chunks.set(ArchChunk { arch: id.arch, sparse });
                chunk = self.chunks.get_mut_(id.arch.get());
                self.archs.push(id.arch.get() as u64);
            }
            unsafe { &mut *chunk.sparse }.set(EntityRef { id: id.id, data });
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    #[repr(C)]
    struct EntityRef<Data> {
        id: NonZeroU16,
        data: Data,
    }

    impl<T: Default> Default for EntityRef<T> {
        fn default() -> Self {
            EntityRef { id: unsafe { NonZeroU16::new_unchecked(u16::MAX) }, data: T::default() }
        }
    }

    impl<T: Default> SparseSlot for EntityRef<T> {
        fn get_id(&self) -> NonZeroU16 {
            self.id
        }
    }

    struct ArchChunk<Data: Default> {
        arch: NonZeroU16,
        sparse: *mut SparseArray<EntityRef<Data>>,
    }

    impl<T: Default> SparseSlot for ArchChunk<T> {
        fn get_id(&self) -> NonZeroU16 {
            self.arch
        }
    }

    impl<T: Default> Default for ArchChunk<T> {
        fn default() -> Self {
            let sparse = Box::into_raw(Box::new(SparseArray::default()));
            Self { arch: unsafe { NonZeroU16::new_unchecked(u16::MAX) }, sparse }
        }
    }

    macro_rules! create_index {
        ($val:ident) => {
            paste::paste! {
                trait [<$val Index>] {
                    fn id<'a, 'b>(&'a self, collection: &'b Components<$val>) -> &'b $val;
                }

                impl<I: [<$val Index>]> Index<I> for Components<$val> {
                    type Output = $val;

                    fn index(&self, index: I) -> &Self::Output {
                        index.id(self)
                    }
                }
            }
        };
    }

    create_index!(Position);
    create_index!(Stats);
    create_index!(Hero);

    let mut rng = WyRand::new();
    let total_entities = u16::MAX - 3;
    let ids: Vec<u16> = (1..=total_entities).map(|i| i).collect();
    let magick_constant = 123123123123123123123123123;

    // ====================
    // ==== Archetypes ====
    // ====================

    let mut Position: Components<Position> =
        Components { len: 0, chunks: SparseArray::default(), archs: Vec::new(), bump: Bump::new() };
    let mut Stats: Components<Stats> =
        Components { len: 0, chunks: SparseArray::default(), archs: Vec::new(), bump: Bump::new() };
    let mut Hero: Components<Hero> =
        Components { len: 0, chunks: SparseArray::default(), archs: Vec::new(), bump: Bump::new() };

    let now = Instant::now();
    let mut entity_id;
    let mut total_arch_entities = 0;
    let arch_sets = 6;
    let entities_per_arch = total_entities / arch_sets;
    let mut arch_expected_result: i128 = 0;
    for _ in 1..7 {
        let arch_id = rng.generate_range(1u16..=entities_per_arch);
        for id in 1..entities_per_arch {
            entity_id = EntityId {
                id: unsafe { NonZeroU16::new_unchecked(id) },
                arch: unsafe { NonZeroU16::new_unchecked(arch_id) },
                arch_offset: 0,
            };
            Position.push(Position { x: id as i32, y: id as i32 }, entity_id);
            Stats.push(Stats { hp: id as i32, damage: id as i32 }, entity_id);
            Hero.push(Hero { name: magick_constant }, entity_id);
            total_arch_entities += 1;
            arch_expected_result += magick_constant + (id + id) as i128;
        }
    }
    let arch_insertion_time = now.elapsed().as_micros();

    println!("");
    println!("Archetypes entties  : {}", total_arch_entities);
    println!("Archetypes inserting: {}micro", arch_insertion_time);
    println!("Archetypes expected : {}", arch_expected_result);

    sleep(Duration::from_millis(2010));

    let now = Instant::now();
    let mut result1 = 0;
    let mut result2 = 0;

    // WHAT NEXT ???
    // - normalizing hash on treshold after deleting
    // - Test for features above
    //
    // - query object
    // - interface that provides propper borrowing rules for iterating values
    // - interface for systtems and dependect fetch
    // - component in world register
    //
    //
    // - iterting macro
    // - resource macro
    //
    //

    // ??? this is impossible because Position now behinde share reference
    let mut iter = &Position.archs;
    iter = if Position.archs.len() < iter.len() { &Position.archs } else { iter };
    iter = if Hero.archs.len() < iter.len() { &Hero.archs } else { iter };
    iter = if Stats.archs.len() < iter.len() { &Stats.archs } else { iter };

    for arch_index in iter {
        let arch_id = unsafe { NonZeroU16::new_unchecked(*arch_index as u16) };

        let position_arch = Position.chunks.get_mut_(arch_id.get());
        let stast_arch = Stats.chunks.get_mut_(arch_id.get());
        let hero_arch = Hero.chunks.get_(arch_id.get());

        let valid_arch = position_arch.arch == arch_id
            && stast_arch.arch == arch_id
            && hero_arch.arch == arch_id;

        let position_sprase = position_arch.sparse;
        let stast_sparse = stast_arch.sparse;
        let hero_sprase = hero_arch.sparse;

        let last_index = unsafe {
            (&*position_sprase)
                .data_slots_end
                .min((&*stast_sparse).data_slots_end)
                .min((&*stast_sparse).data_slots_end)
        };

        if valid_arch {
            let max = last_index / BITS_SIZE;

            for chunk in 0usize..=max as usize {
                let entity_index_offset = chunk * BITS_SIZE as usize;
                let mut sv = [0u16; BITS_SIZE as usize];
                let start_index = if chunk == 0 { 1 } else { 0 };
                let mut len = start_index;

                let value: BitsChunk = unsafe {
                    (*position_sprase).bits.get_unchecked(chunk)
                        & (*stast_sparse).bits.get_unchecked(chunk)
                        & (*hero_sprase).bits.get_unchecked(chunk)
                };

                for i in 0..BITS_SIZE as usize {
                    if (value & (1 << i)) != 0 {
                        sv[i] = (entity_index_offset + i) as u16;
                        len += 1;
                        if (i % 8) == 0 {
                            unsafe {
                                (*position_sprase).prefetch_read((entity_index_offset + i) as u16);
                                (*stast_sparse).prefetch_read((entity_index_offset + i) as u16);
                                (*hero_sprase).prefetch_read((entity_index_offset + i) as u16);
                            }
                        }
                    }
                }

                // With MAX RUST could optimize
                // 0..BITS_SIZE, is 10% faster than conditional cycle
                // instead 0..63, (max * 64)..(position_arch.sparse.data_last_index) may be a special cases
                // before and after cycle
                for i in start_index..len.min(BITS_SIZE) {
                    let id = sv[i as usize];
                    let pos = unsafe { (*position_sprase).get_(id) };
                    let stat = unsafe { (*stast_sparse).get_(id) };
                    let hero = unsafe { (*hero_sprase).get_(id) };
                    // let add = (pos.data.x + stat.data.damage) as i128 + hero.data.name;
                    // if position_arch.sparse.get_(id).data.x == 0 {
                    //     println!("Value in index {}: {} {}", id, add, (add - magick_constant) / 2);
                    //     println!("Value {:?}", position_arch.sparse.get_(id));
                    // }
                    // if let Some(ref mut set) = all_entities.get_mut(&arch_id.get()) {
                    //     if !set.remove(&((pos.data.x + stat.data.damage) as i128 + hero.data.name)) {
                    //         // println!("removed multiple time {}", id);
                    //     }
                    // }
                    result1 += 1;
                    result2 += (pos.data.x + stat.data.damage) as i128 + hero.data.name;
                }
            }
        }
    }

    let r = now.elapsed().as_micros();
    println!("Archetypes result   : {:?}", result2);
    println!("Archetypes entities : {:?}", result1);
    println!("Archetypes time     : {}micro", r);
    // println!(
    //     "{:?}",
    //     all_entities
    //         .iter()
    //         .flat_map(|(_, x)| x.iter().map(|x| (x - 123123123123123123123123123) / 2))
    //         .collect::<Vec<i128>>()
    // );

    return 0;
}

// average iteration times around 510
fn iter_shipayrd() {
    use shipyard::{Component, IntoIter, View, World};

    #[derive(Component)]
    struct ShipStats {
        damage: i32,
        health: i32,
    }

    #[derive(Component)]
    struct ShipPosition {
        x: i32,
        y: i32,
    }

    #[derive(Component)]
    struct ShipHero {
        name: i128,
    }

    fn collect(positions: View<ShipPosition>, hero: View<ShipHero>, healths: View<ShipStats>) {
        let mut result: (u64, i128) = (0, 0);
        let now = Instant::now();
        for (pos, hero, stats) in (&positions, &hero, &healths).iter() {
            result.0 += 1;
            result.1 += (pos.x + stats.damage) as i128 + hero.name;
        }
        let r = now.elapsed().as_micros();

        println!("Shipyard iterations : {:?}", result.0);
        println!("Shipyard time       : {}micro", r);
        println!("Shipyard result     : {:?}", result.1);
    }

    let mut rng = WyRand::new();
    let total_entities = u16::MAX;
    let ids: Vec<u16> = (0..=total_entities).map(|i| i).collect();
    let magick_constant = 123123123123123123123123123;

    // =====================
    // ===== Shipayard =====
    // =====================

    let mut world = World::new();
    let mut shipyard_ids: Vec<shipyard::EntityId> = Vec::new();
    let mut ship_expected_result: i128 = 0;
    let now = Instant::now();
    // for id in &ids {
    //     shipyard_ids.push(world.add_entity((
    //         ShipPosition {
    //             x: *id as i32,
    //             y: *id as i32,
    //         },
    //         ShipStats {
    //             damage: *id as i32,
    //             health: *id as i32,
    //         },
    //         ShipHero { name: magick_constant },
    //     )));
    //     ship_expected_result += *id as i128;
    // }
    for id in &ids {
        shipyard_ids.push(world.add_entity((ShipPosition { x: 10 as i32, y: 10 as i32 },)));
        ship_expected_result += *id as i128;
    }
    let i = 0;
    shipyard_ids.sort_unstable_by(|_, _| {
        rng.generate_range(1u64..=100).cmp(&rng.generate_range(1u64..=100))
    });
    for id in shipyard_ids.iter() {
        world.add_component(*id, ShipStats { damage: 10 as i32, health: 10 as i32 });
        ship_expected_result += ids[i] as i128;
    }
    shipyard_ids.sort_unstable_by(|_, _| {
        rng.generate_range(1u64..=100).cmp(&rng.generate_range(1u64..=100))
    });
    for id in shipyard_ids.into_iter() {
        world.add_component(id, ShipHero { name: magick_constant });
        ship_expected_result += magick_constant;
    }
    let shipyard_insertion = now.elapsed().as_micros();

    sleep(Duration::from_millis(2010));

    println!("");
    world.run(collect);
    println!("Shipyard insertion  : {}micro", shipyard_insertion)
}
