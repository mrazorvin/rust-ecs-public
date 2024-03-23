pub mod collections;
pub mod components;
pub mod devtools;
pub mod entity;
pub mod frame_dispose;
pub mod integration_tests;
pub mod logger;
pub mod macros;
pub mod query;
pub mod system;
pub mod testing;
pub mod tests;
pub mod view;
pub mod world;

pub mod ecs_mode {
    pub enum Unknown {}

    // ecs executed only from main i.e access to shared resource is safe
    pub enum Exclusive {}

    // ecs could execute more than one system i.e access to exclusive data could be ony read only
    pub enum Shared {}
}

pub mod prelude {
    pub use super::components::Components;
    pub use super::ecs_mode;
    pub use super::entity;
    pub use super::frame_dispose;
    pub use super::query::*;
    pub use super::system;
    pub use super::view::*;
    pub use super::world;
    pub use super::world::Schedule;
    pub use super::world::UniqueResource;
}

pub use integration_tests::integration_tests;
