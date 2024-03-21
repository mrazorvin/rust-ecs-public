use crate::ecs::{
    collections::sync_ivec::PREVIOUS_SNAPSHOTS,
    components::CHANGED_COMPONENTS,
    world::{self, World},
};

pub fn disposer(world: &mut World) {
    for chunk in CHANGED_COMPONENTS.chunks() {
        for i in 0..chunk.len() {
            if let Some((_, ref mut resource)) = world.resources.get_mut(&chunk[i]) {
                resource.dispose_frame();
            }
        }
    }
    unsafe { CHANGED_COMPONENTS.reset() }

    for chunk in PREVIOUS_SNAPSHOTS.chunks() {
        for i in 0..chunk.len() {
            unsafe { (*chunk[i].data).dispose() }
        }
    }
    unsafe { PREVIOUS_SNAPSHOTS.reset() }
}
