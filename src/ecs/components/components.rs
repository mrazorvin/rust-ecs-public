use crate::ecs::collections::{
    sync_sparse_array::{self, sync_array},
    sync_sparse_chunked_store::{SyncSparseArrayChunk, SyncSparseChunkedStore},
    sync_vec::SyncVec,
};
use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

// this and possible some other functions required 
// additional trait bound than just any
// for example disposing/is_changed etc... 
pub static CHANGED_COMPONENTS: SyncVec<TypeId> = SyncVec::new();

pub struct Components<T> {
    store: SyncSparseChunkedStore<T>,
    len: AtomicU32,

    // must be there because of send/sync/copy/debug and other auto-traits that may be overwritten by `SyncSparseChunkedStore<T>`
    _marker: PhantomData<T>,
}

impl<T: 'static> Components<T> {
    pub unsafe fn try_set(&self, arch_id: usize, entity_id: usize, data: T) {
        if self.store_set(arch_id, entity_id, data).is_none() {
            self.len.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn store_set(&self, chunk_id: usize, entry_id: usize, data: T) -> Option<T> {
        let chunk = self.chunks.get_or_insert(chunk_id, &|| {
            (
                chunk_id,
                SyncSparseArrayChunk {
                    arr: Box::into_raw(Box::new(sync_array(Some(self as *const Components<T>)))),
                },
            )
        });

        chunk.1.set_in_place(entry_id, data)
    }

    pub fn set(&mut self, arch_id: usize, entity_id: usize, data: T) {
        unsafe { self.try_set(arch_id, entity_id, data) };
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed) as usize
    }
}

impl<T: 'static> Default for Components<T> {
    fn default() -> Self {
        Self {
            len: AtomicU32::new(0),
            store: SyncSparseChunkedStore::new(),
            _marker: PhantomData {},
        }
    }
}

impl<T> Deref for Components<T> {
    type Target = SyncSparseChunkedStore<T>;
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl<T> DerefMut for Components<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}
