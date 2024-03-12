use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::ecs::{
    collections::sync_sparse_array::{BucketRef, BucketRefMut, SparseBucketRaw, SyncSparseArray},
    components::Components,
};

// #region ### View types

#[allow(non_camel_case_types)]
pub struct read {}

#[allow(non_camel_case_types)]
pub struct write {}
// #endregion

/// View provides safe interface for Components store operations  
/// but doesn't synchronze those operations between threads.
///
/// Thats why creating and exposing view to public access is unsafe
/// but calling other methods is safe
#[repr(transparent)]
pub struct View<T, U = read> {
    data: *mut T,
    _marker: PhantomData<U>,
}

// #region ### View - shared
impl<T, U> View<T, U> {
    pub unsafe fn new(components: *mut T) -> View<T, U> {
        View {
            data: components,
            _marker: PhantomData {},
        }
    }

    #[inline]
    pub unsafe fn data_ptr(&self) -> *mut T {
        self.data
    }
}

impl<T, U> Deref for View<T, U> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}
//
impl<T, U> View<Components<T>, U> {}

impl<T> View<Components<T>, read> {
    pub unsafe fn new_read(components: *mut T) -> View<T, read> {
        View {
            data: components,
            _marker: PhantomData {},
        }
    }

    #[inline]
    pub unsafe fn get_bucket(
        &self,
        sparse_vec: &SyncSparseArray<T>,
        bucket_ref: &SparseBucketRaw<T>,
    ) -> BucketRef<T> {
        unsafe { sparse_vec.bucket_from_ref(bucket_ref) }
    }
}

impl<T> View<Components<T>, write> {
    pub unsafe fn new_write(components: *mut T) -> View<T, write> {
        View {
            data: components,
            _marker: PhantomData {},
        }
    }

    #[inline]
    pub unsafe fn get_bucket(
        &self,
        sparse_vec: &SyncSparseArray<T>,
        bucket_ref: &SparseBucketRaw<T>,
    ) -> BucketRefMut<T> {
        unsafe { sparse_vec.bucket_lock_from_ref(bucket_ref) }
    }
}

// #endregion

// #region ### View - mutable
impl<T> DerefMut for View<T, write> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}
// #endregion
