use crate::ecs::entity::entity_id::EntityId;

pub mod disposable;
pub mod primes_u16;
pub mod raw_vec;
pub mod sparse_array;
pub mod sync_ivec;
pub mod sync_sparse_array;
pub mod sync_sparse_chunked_store;
pub mod sync_vec;

pub const BUCKET_SIZE: usize = 256 / core::mem::size_of::<EntityId>();
