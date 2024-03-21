use super::sync_vec::SyncVec;
use std::{
    num::NonZeroU32,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering},
};

#[repr(transparent)]
struct SyncSlot {
    key: NonZeroU32,
}

struct FreeKeysMeta {
    store_iter: AtomicU32,
    load_iter: AtomicU32,
}

struct SyncSlotMap<T> {
    free_keys_store_idx: AtomicU32,
    free_keys_meta: [FreeKeysMeta; 2],
    free_keys: [SyncVec<u32>; 2],
    slots: SyncVec<T>,
}

fn get_keys_index(value: u32) -> usize {
    (value % 2) as usize
}

impl<T> SyncSlotMap<T> {
    pub fn new() -> SyncSlotMap<T> {
        SyncSlotMap {
            free_keys_store_idx: AtomicU32::new(0),
            free_keys: [SyncVec::new(), SyncVec::new()],
            free_keys_meta: [
                FreeKeysMeta { load_iter: AtomicU32::new(0), store_iter: AtomicU32::new(0) },
                FreeKeysMeta { load_iter: AtomicU32::new(0), store_iter: AtomicU32::new(0) },
            ],
            slots: SyncVec::new(),
        }
    }

    pub fn push(&self) -> SyncSlot {
        SyncSlot { key: todo!() }
    }

    pub fn get_key(&self) -> Option<u32> {
        // the problem that we could change buffer not related to update
        // for example buffer was swapped by another thread
        // thats why we should, be carefully, oor least be sure that changing bad buffer won't cause a lot of problem

        // 1. increasing max by 1 for bad buffer, only take one additional slot, nothig more, so it's probably safe because we just skip one empty slot, even if it's alredy used, in worth keys we could lose single key because there we will interate only up to max
        // 2. increasing current for is worst because we have hihger chnace to skip keys, because we use this as for filtering
        loop {
            let (store_key, store_vec, store_meta) = self.get_load_vec();
            let (load_key, load_store, load_meta) = self.get_store_vec();

            // load vec is drained
            if load_meta
                .store_iter
                .load(Ordering::Acquire)
                .saturating_sub(load_meta.load_iter.load(Ordering::Acquire))
                == 0
            {
                // no free keys in store vec
                if store_meta.store_iter.load(Ordering::Acquire) == 0 {
                    break None;
                } else {
                    // doesn't matter if 
                    let _ = self.free_keys_store_idx.compare_exchange(
                        store_key,
                        load_key,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    );
                    continue;
                }
            }

            // load vec is drained and not free keys in store vec
            if store_meta.store_iter.load(Ordering::Acquire) == 0
                && load_meta
                    .store_iter
                    .load(Ordering::Acquire)
                    .saturating_sub(load_meta.load_iter.load(Ordering::Acquire))
                    == 0
            {}

            // iterate over current with rnage iter and find first non-zero key
            // compare-swap with zero, current index by 1, ^ comare cur with last in condition above
            // return value from this index
        }
    }

    pub fn get_load_vec(&self) -> (u32, &SyncVec<u32>, &FreeKeysMeta) {
        let cur_key = self.free_keys_store_idx.load(Ordering::Acquire);
        unsafe {
            (
                cur_key,
                self.free_keys.get_unchecked(get_keys_index(cur_key)),
                self.free_keys_meta.get_unchecked(get_keys_index(cur_key)),
            )
        }
    }

    pub fn get_store_vec(&self) -> (u32, &SyncVec<u32>, &FreeKeysMeta) {
        let next_key = self.free_keys_store_idx.load(Ordering::Acquire) + 1;
        unsafe {
            (
                next_key,
                self.free_keys.get_unchecked(get_keys_index(next_key)),
                self.free_keys_meta.get_unchecked(get_keys_index(next_key)),
            )
        }
    }
}
