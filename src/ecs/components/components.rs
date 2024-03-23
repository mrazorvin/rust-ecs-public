use crate::ecs::{
    collections::{
        sync_sparse_array::{self, get_bucket_index, sync_array, SyncSparseArray, BUCKET_DENSITY},
        sync_sparse_chunked_store::{SyncSparseArrayChunk, SyncSparseChunkedStore},
        sync_vec::{SyncVec, ZipRangeIterator},
    },
    ecs_mode,
    system::{self, sys_mode, Stage},
    world,
};
use core::arch;
use std::{
    any::{self, TypeId},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

// this and possible some other functions required
// additional trait bound than just any
// for example disposing/is_changed etc...

pub static XX: Vec<TypeId> = Vec::new();

pub static CHANGED_COMPONENTS: SyncVec<TypeId> = SyncVec::new();

pub struct Components<T: Sized> {
    pub is_updated: AtomicBool,
    store: SyncSparseChunkedStore<T>,
    len: AtomicU32,
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

    pub fn touch(&self) {
        if !self.is_updated.load(Ordering::Relaxed) {
            if let Ok(_) =
                self.is_updated.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            {
                CHANGED_COMPONENTS.push(TypeId::of::<Self>());
            }
        }
    }
}

impl<T: 'static> Default for Components<T> {
    fn default() -> Self {
        Self {
            len: AtomicU32::new(0),
            store: SyncSparseChunkedStore::new(),
            is_updated: AtomicBool::new(false),
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

pub trait ComponnetsResource: Sized + 'static {
    fn dispose_frame(_components: &mut Components<Self>) -> bool {
        true
    }

    fn fetch(
        system: &mut system::State<ecs_mode::Exclusive, sys_mode::Configuration>,
    ) -> Result<*mut Components<Self>, String> {
        let components_ptr = unsafe { system.world() }
            .resources
            .get(&TypeId::of::<Components<Self>>())
            .map(|(ptr, _)| unsafe { std::mem::transmute::<*mut u8, *mut Components<Self>>(*ptr) });

        match system.stage() {
            Stage::Instantination => Err(format!(
                "Instantination stage can't be used to fetch {}",
                any::type_name::<Components<Self>>()
            )),
            Stage::Initialization => match components_ptr {
                Some(ptr) => Ok(ptr),
                None => {
                    let comps: Components<Self> = Components::default();
                    unsafe { system.world() }.add_resource(comps)
                }
            },
            Stage::Execution => components_ptr
                .ok_or(format!("{} non exists in world", any::type_name::<Components<Self>>())),
        }
    }
}

impl<T: ComponnetsResource> world::Resource for Components<T> {}
impl<T: ComponnetsResource> world::DisposeFrame for Components<T> {
    fn dispose_frame(&mut self) -> bool {
        if !T::dispose_frame(self) || !*self.is_updated.get_mut() {
            return false;
        }

        *self.is_updated.get_mut() = false;
        for (_, arch) in unsafe { self.chunks.as_slice() } {
            if arch.is_updated.load(Ordering::Relaxed) {
                let mut arch_iter = ZipRangeIterator::new();
                let mut bucket_iter = &mut arch_iter.add(
                    &arch.buckets,
                    arch.min_updated_id.load(Ordering::Relaxed) as usize,
                    arch.max_updated_id.load(Ordering::Relaxed) as usize,
                );

                let max_bits = arch.max_relaxed();
                if max_bits != u32::MIN as usize {
                    let min_bits = arch.min_relaxed();
                    let min_bits_bucket = get_bucket_index(min_bits);
                    let max_bits_bucket = get_bucket_index(max_bits);

                    if min_bits_bucket != max_bits_bucket {
                        if unsafe { arch.get_bits_unchecked(min_bits) == 0 } {
                            arch.min_existed_id.store(
                                (min_bits_bucket + BUCKET_DENSITY) as u32,
                                Ordering::Relaxed,
                            );
                        }
                        if unsafe { arch.get_bits_unchecked(max_bits) == 0 } {
                            arch.max_existed_id.store(
                                (max_bits_bucket - BUCKET_DENSITY) as u32,
                                Ordering::Relaxed,
                            );
                        }
                    }
                }

                for mut chunk in arch_iter {
                    let bucket = chunk.progress(&mut bucket_iter);
                    for i in chunk.complete() {
                        let (_, updated_bits, _) = &bucket[i];
                        if updated_bits.load(Ordering::Relaxed) != 0 {
                            updated_bits.store(0, Ordering::Relaxed);
                        }
                    }
                }

                // arch.last_updated = GAMESESSION_CYCLE;
                arch.min_updated_id.store(u32::MAX, Ordering::Relaxed);
                arch.max_updated_id.store(u32::MIN, Ordering::Relaxed);
                arch.is_updated.store(false, Ordering::Relaxed);
            }
        }

        true
    }
}

#[test]
fn components_dispose() {
    impl ComponnetsResource for Box<u32> {}

    let mut components = Components::default();

    components.set(1, 109, Box::new(u32::MAX));
    components.set(1, 253, Box::new(u32::MIN));

    assert_eq!(*components.is_updated.get_mut(), true);
    let (_, arch) = components.chunks.get_or_insert(1, &|| unimplemented!());

    assert_eq!(arch.min_relaxed(), 64);
    assert_eq!(arch.max_relaxed(), 255);
    assert_eq!(arch.min_updated_id.load(Ordering::Relaxed), 64);
    assert_eq!(arch.max_updated_id.load(Ordering::Relaxed), 255);
    assert_eq!(arch.is_updated.load(Ordering::Relaxed), true);

    world::DisposeFrame::dispose_frame(&mut components);
    assert_eq!(*components.is_updated.get_mut(), false);
    let (_, arch) = components.chunks.get_or_insert(1, &|| unimplemented!());

    assert_eq!(arch.min_relaxed(), 64);
    assert_eq!(arch.max_relaxed(), 255);
    assert_eq!(arch.min_updated_id.load(Ordering::Relaxed), u32::MAX);
    assert_eq!(arch.max_updated_id.load(Ordering::Relaxed), u32::MIN);
    assert_eq!(arch.is_updated.load(Ordering::Relaxed), false);
}
