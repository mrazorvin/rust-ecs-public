use crate::ecs::collections::sync_sparse_chunked_store::SyncSparseChunkedStore;
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

pub struct Components<T> {
    store: SyncSparseChunkedStore<T>,
    len: AtomicU32,

    // must be there because of send/sync/copy/debug and other auto-traits that may be overwritten by `SyncSparseChunkedStore<T>`
    _marker: PhantomData<T>,
}

impl<T: 'static> Components<T> {
    pub unsafe fn try_set(&self, arch_id: usize, entity_id: usize, data: T) {
        if self.store.set(arch_id, entity_id, data).is_none() {
            self.len.fetch_add(1, Ordering::Relaxed);
        }
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
