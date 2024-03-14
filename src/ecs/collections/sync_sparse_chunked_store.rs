use std::ops::Deref;

use super::{sync_ivec::IVec, sync_sparse_array::SyncSparseArray};

pub struct SyncSparseChunkedStore<Data> {
    pub chunks: IVec<(usize, SyncSparseArrayChunk<Data>)>,
}

impl<T: 'static> SyncSparseChunkedStore<T> {
    pub fn new() -> Self {
        Self { chunks: IVec::new() }
    }
}

// #region ### chunk wrapper that compatibily with ivec bounds

#[repr(transparent)]
pub struct SyncSparseArrayChunk<T> {
    pub arr: *mut SyncSparseArray<T>,
}

unsafe impl<T> Sync for SyncSparseArrayChunk<T> {}
unsafe impl<T> Send for SyncSparseArrayChunk<T> {}

impl<T> Clone for SyncSparseArrayChunk<T> {
    fn clone(&self) -> Self {
        SyncSparseArrayChunk { arr: self.arr }
    }
}

impl<T> Drop for SyncSparseArrayChunk<T> {
    fn drop(&mut self) {
        unsafe { drop(Box::from_raw(self.arr)) };
    }
}

impl<T> Deref for SyncSparseArrayChunk<T> {
    type Target = SyncSparseArray<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.arr }
    }
}
// #endregion
