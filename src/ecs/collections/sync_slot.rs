use super::sync_vec::{SyncVec, ZipRangeIterator};
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
    free_keys: [SyncVec<AtomicU32>; 2],
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
        'lookup_free_key: loop {
            let (load_key, load_vec, load_meta) = self.get_load_vec();
            let (store_key, _, store_meta) = self.get_store_vec();

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
                }
                // swap vecs
                else {
                    if let Ok(_) = self.free_keys_store_idx.compare_exchange(
                        load_key,
                        store_key,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    ) {
                        // reset counters to 0
                        store_meta.load_iter.store(0, Ordering::Release);
                        load_meta.store_iter.store(0, Ordering::Release);
                    }

                    // doesn't matter if change was successful or not in both caseses we must repeat process again
                    continue 'lookup_free_key;
                }
            }

            let mut iter = ZipRangeIterator::new();
            let mut load_iter = &mut iter.add(
                &load_vec,
                load_meta.load_iter.load(Ordering::Acquire) as usize,
                u32::MAX as usize,
            );
            for mut chunk in iter {
                let vec = chunk.progress(&mut load_iter);
                for i in chunk.complete() {
                   if vec[i].load(Ordering::Acquire) == 0 {

                   }
                }
            }
        }
    }

    pub fn get_load_vec(&self) -> (u32, &SyncVec<AtomicU32>, &FreeKeysMeta) {
        let cur_key = self.free_keys_store_idx.load(Ordering::Acquire);
        unsafe {
            (
                cur_key,
                self.free_keys.get_unchecked(get_keys_index(cur_key)),
                self.free_keys_meta.get_unchecked(get_keys_index(cur_key)),
            )
        }
    }

    pub fn get_store_vec(&self) -> (u32, &SyncVec<AtomicU32>, &FreeKeysMeta) {
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
