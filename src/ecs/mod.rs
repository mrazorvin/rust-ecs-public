pub mod collections;
pub mod components;
pub mod devtools;
pub mod entity;
pub mod integration_tests;
pub mod logger;
pub mod macros;
pub mod query;
pub mod system;
pub mod testing;
pub mod tests;
pub mod view;
pub mod world;

pub mod prelude {
    pub use super::components::Components;
    pub use super::entity;
    #[allow(unused_imports)]
    pub use super::query::*;
    pub use super::system;
    pub use super::view::*;
    pub use super::world;
    pub use super::world::Schedule;
    pub use super::world::UniqueResource;
}

pub use integration_tests::integration_tests;
