use std::{
    cell::UnsafeCell,
    fmt::Debug,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering},
        Mutex, RwLock,
    },
    usize,
};

use super::sync_vec::SyncVec;

// Total amount of buckets per single array
// single bucket occupied 8bytes of space,
// 128 bucket == 1kb of space
const BUCKETS_PER_ARRAY: usize = 128; // value must be module of 2
const BITS_PER_BUCKET: usize = u64::BITS as usize;

// Total amount of items that could be stored per bucket
const BUCKET_DENSITY: usize = BITS_PER_BUCKET; // Currently this value MUST equals to bits count in AutomicU64

// Default size of every array should be ~ 1kb
// first 4096 elements is stored in chunks by 64 items
// following 6000k is stored in chunks for 960 items
//
// Maximum amount of items that could be stored in array, must be less than u16::MAX
// with total 128 buckets and START_BUCKET_DENSITY = 64, END_BUCKET_DENSITY, this values
// equals to u16:MAX
const MAX_ITEMS_PER_ARRAY: usize = BUCKETS_PER_ARRAY * BUCKET_DENSITY;

pub fn get_slot_index(id: usize) -> usize {
    id % BUCKET_DENSITY
}

pub fn get_bucket_index(id: usize) -> usize {
    id / BUCKET_DENSITY
}

// basically we could have timed bucket or simpel bucket

pub type SparseBucket<T> = (AtomicU64, AtomicPtr<Bucket<T>>);
pub type SparseBucketRaw<T> = (UnsafeCell<u64>, *mut Bucket<T>);

pub struct SyncSparseArray<T> {
    pub buckets: SyncVec<SparseBucket<T>>,
    pub min_existed_id: AtomicU32,
    pub max_existed_id: AtomicU32,
    pub min_updated_id: AtomicU32,
    pub max_updated_id: AtomicU32,
}

pub fn sync_array<T>() -> SyncSparseArray<T> {
    SyncSparseArray {
        buckets: SyncVec::new(),
        max_updated_id: AtomicU32::new(u32::MIN),
        min_existed_id: AtomicU32::new(u32::MAX),
        max_existed_id: AtomicU32::new(u32::MIN),
        min_updated_id: AtomicU32::new(u32::MAX),
    }
}

impl<T> SyncSparseArray<T> {
    pub fn new() -> Self {
        SyncSparseArray {
            max_updated_id: AtomicU32::new(u32::MIN),
            min_existed_id: AtomicU32::new(u32::MAX),
            max_existed_id: AtomicU32::new(u32::MIN),
            min_updated_id: AtomicU32::new(u32::MAX),
            buckets: SyncVec::new(),
        }
    }

    /// safety: self.buckets.get() is always safe to call, but following
    ///         transformation is unsafe because without manual sheduling
    ///         user may cause data races
    unsafe fn bucket_raw_unchecked(&self, id: usize) -> Option<&SparseBucketRaw<T>> {
        unsafe { std::mem::transmute(self.buckets.get(get_bucket_index(id))) }
    }

    /// the content of fucntion is safe but, because it's return raw bucket representation
    /// it's unsafe, check description for `bucket_raw_unchecked` to get why raw bucket is usnafe
    unsafe fn get_raw_bucket_or_create(&self, id: usize) -> &SparseBucketRaw<T> {
        match self.bucket_raw_unchecked(id) {
            Some(bucket_ref) => bucket_ref,
            None => {
                let requested_index = (id as f32 / BUCKET_DENSITY as f32).ceil() as usize;
                while self.buckets.size() <= requested_index {
                    let bucket_ptr =
                        Box::into_raw(Box::new(Bucket::new(get_bucket_index(id) as u32)));
                    self.buckets
                        .push((AtomicU64::new(0), AtomicPtr::new(bucket_ptr)));
                }

                // sice requested bucket could be created by other thread,
                // we must find it, instead of relaying on latest created bucket
                self.bucket_raw_unchecked(id).unwrap()
            }
        }
    }

    pub unsafe fn get_bits_unchecked(&self, id: usize) -> u64 {
        self.buckets
            .get_unchecked(get_bucket_index(id))
            .0
            .load(Ordering::Acquire)
    }

    pub fn bucket_lock(&self, id: usize) -> BucketRefMut<T> {
        unsafe { self.bucket_lock_from_ref(self.get_raw_bucket_or_create(id)) }
    }

    /**
     * Returns bucket lock that safe to mutate in single thread
     *
     * NOTE: if bucket not exists yet, it will be created on demand
     *       don't use this method to check value presence, if
     *       you don't want to empty buckets everywhere
     */
    pub unsafe fn bucket_lock_from_ref(&self, bucket_ref: &SparseBucketRaw<T>) -> BucketRefMut<T> {
        let bucket_ptr = bucket_ref.1;

        while unsafe { (*bucket_ptr).guard.load(Ordering::Relaxed) } {
            std::hint::spin_loop()
        }

        while unsafe {
            (*bucket_ptr)
                .guard
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
        } {}

        BucketRefMut {
            array: self as *const _,
            bucket: bucket_ptr,
            bits: bucket_ref.0.get(),
        }
    }

    // this function is unsafe because normally you should get reference
    // to bucket without locking, but because we are using double buffering it possible
    pub unsafe fn bucket(&self, id: usize) -> BucketRef<T> {
        self.bucket_from_ref(self.bucket_raw_unchecked(id).unwrap())
    }

    // this function is unsafe because normally you should get reference
    // to bucket without locking, but because we are using double buffering it possible
    pub unsafe fn bucket_from_ref(&self, bucket_ref: &SparseBucketRaw<T>) -> BucketRef<T> {
        BucketRef {
            bits: bucket_ref.0.get(),
            bucket_ptr: bucket_ref.1,
        }
    }

    pub fn delete_in_place(&self, id: usize) -> Option<T> {
        self.bucket_lock(id).delete(id)
    }

    pub fn set_in_place(&self, id: usize, data: T) -> Option<T> {
        self.bucket_lock(id).set(id, data)
    }

    pub fn min_relaxed(&self) -> usize {
        self.min_existed_id.load(Ordering::Relaxed) as usize
    }

    pub fn max_relaxed(&self) -> usize {
        self.max_existed_id.load(Ordering::Relaxed) as usize
    }
}

impl<T> Drop for SyncSparseArray<T> {
    fn drop(&mut self) {
        for chunk in self.buckets.chunks() {
            for i in 0..chunk.len() {
                let bucket_ref = &chunk[i];
                let bucket_ptr = bucket_ref.1.load(Ordering::Acquire);
                let bits = bucket_ref.0.load(Ordering::Acquire);

                for bit_index in 0..BUCKET_DENSITY {
                    if (bits & (1 << bit_index)) != 0 {
                        let value = std::mem::replace(
                            unsafe { &mut ((*bucket_ptr).slots[bit_index]) },
                            MaybeUninit::uninit(),
                        );
                        unsafe { drop(value.assume_init()) };
                    }
                }

                if !bucket_ptr.is_null() {
                    unsafe { drop(Box::from_raw(bucket_ptr)) };
                }
            }
        }
    }
}

pub struct BucketRef<T> {
    bits: *mut u64,
    bucket_ptr: *mut Bucket<T>,
}

impl<T> BucketRef<T> {
    #[allow(dead_code)]
    pub fn bits(&self) -> u64 {
        unsafe { *self.bits }
    }

    pub unsafe fn get_unchecked(&self, id: usize) -> &T {
        let result =
            (*(*self.bucket_ptr).slots.get_unchecked(get_slot_index(id))).assume_init_ref();

        return result;
    }
}

pub struct BucketRefMut<T> {
    array: *const SyncSparseArray<T>,
    bucket: *mut Bucket<T>,
    bits: *mut u64,
}

impl<T> BucketRefMut<T> {
    #[allow(dead_code)]
    pub fn bits(&self) -> u64 {
        unsafe { *self.bits }
    }

    pub fn set(&mut self, id: usize, data: T) -> Option<T> {
        assert!(id < MAX_ITEMS_PER_ARRAY);

        let slot_idx = get_slot_index(id);
        let is_new_value = (unsafe { *self.bits } & (1 << slot_idx)) == 0;
        if is_new_value {
            let slot_ref = unsafe { (*self.bucket).slots.get_unchecked_mut(slot_idx) };
            unsafe { (slot_ref as *mut MaybeUninit<T>).write(MaybeUninit::new(data)) }
            unsafe { *self.bits |= 1 << get_slot_index(id) };
            None
        } else {
            let old_value = std::mem::replace(
                unsafe { (*self.bucket).slots.get_unchecked_mut(slot_idx) },
                MaybeUninit::new(data),
            );
            unsafe { Some(old_value.assume_init()) }
        }
    }

    pub fn delete(&mut self, id: usize) -> Option<T> {
        assert!(id < MAX_ITEMS_PER_ARRAY);

        let is_value_exists = (unsafe { *self.bits } & (1 << get_slot_index(id))) != 0;

        if is_value_exists {
            unsafe { *self.bits &= !(1 << get_slot_index(id)) };

            Some(unsafe {
                ((*self.bucket).slots.get_unchecked_mut(get_slot_index(id)) as *mut _ as *mut T)
                    .read()
            })
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn has(&self, id: usize) -> bool {
        unsafe { *self.bits & (1 << get_slot_index(id)) != 0 }
    }

    #[allow(dead_code)]
    pub unsafe fn get_unchecked(&self, id: usize) -> &T {
        (*(*self.bucket).slots.get_unchecked(get_slot_index(id))).assume_init_ref()
    }

    #[allow(dead_code)]
    pub unsafe fn get_mut_unchecked(&mut self, id: usize) -> &mut T {
        (*(*self.bucket).slots.get_unchecked_mut(get_slot_index(id))).assume_init_mut()
    }

    #[allow(dead_code)]
    pub unsafe fn get_unchecked_copy(&self, id: usize) -> T
    where
        T: Copy,
    {
        (*(*self.bucket).slots.get_unchecked(get_slot_index(id))).assume_init()
    }

    #[allow(dead_code)]
    pub fn get_copy(&self, id: usize) -> Option<T>
    where
        T: Copy,
    {
        if self.has(id) {
            Some(unsafe { (*(*self.bucket).slots.get_unchecked(get_slot_index(id))).assume_init() })
        } else {
            None
        }
    }
}

impl<T> Drop for BucketRefMut<T> {
    fn drop(&mut self) {
        if unsafe { *self.bits != 0 } {
            let idx = unsafe { (*self.bucket).idx as usize };
            let min = (idx * BUCKET_DENSITY) as u32;
            let max = ((min as usize + BUCKET_DENSITY) - 1) as u32;
            if unsafe { min < (*self.array).min_existed_id.load(Ordering::Relaxed) } {
                unsafe {
                    (*self.array)
                        .min_existed_id
                        .fetch_min(min, Ordering::AcqRel)
                };
            }
            if unsafe { max > (*self.array).max_existed_id.load(Ordering::Relaxed) } {
                unsafe {
                    (*self.array)
                        .max_existed_id
                        .fetch_max(max, Ordering::AcqRel)
                };
            }
        }

        unsafe { (*self.bucket).guard.store(false, Ordering::Release) };
    }
}

pub struct Bucket<T> {
    guard: AtomicBool,
    idx: u32,
    pub slots: [MaybeUninit<T>; BUCKET_DENSITY],
}

impl<T> Bucket<T> {
    fn new(idx: u32) -> Self {
        Self {
            guard: AtomicBool::new(false),
            idx,
            slots: unsafe { MaybeUninit::zeroed().assume_init() },
        }
    }
}

#[test]
fn test_bucket_lock_api() {
    let id = 5000;

    // #region ### Single insert & impl drop
    let array = sync_array();
    let mut bucket = array.bucket_lock(id);
    assert_eq!(bucket.set(id, String::from("value for id 123")), None);
    assert_eq!(array.min_relaxed(), u32::MAX as usize);
    assert_eq!(array.max_relaxed(), 0);
    assert_eq!(unsafe { array.get_bits_unchecked(id) }, 1 << (id % 64));
    assert!(bucket.has(id));
    drop(bucket);

    // min-max changed only after we drop bucket
    assert_eq!(
        array.min_relaxed(),
        (5000 / BUCKET_DENSITY * BUCKET_DENSITY)
    );
    assert_eq!(
        array.max_relaxed(),
        ((5000 / BUCKET_DENSITY * BUCKET_DENSITY) + BUCKET_DENSITY - 1)
    );

    drop(array);
    // #endregion

    // #region ### Mutltiple insert - manual drop
    let array = sync_array();
    let mut bucket = array.bucket_lock(id);
    assert_eq!(bucket.set(id, String::from("value for id 123")), None);

    let value = unsafe { bucket.get_mut_unchecked(id) };
    value.push_str(" with additional info");

    let value = unsafe { bucket.get_unchecked(id) }.clone();

    assert_eq!(
        Some(value.clone()),
        Some(String::from("value for id 123 with additional info"))
    );
    assert_eq!(bucket.delete(id), Some(value.clone()));
    assert_eq!(array.min_relaxed(), u32::MAX as usize);
    assert_eq!(array.max_relaxed(), 0);
    assert_eq!(unsafe { array.get_bits_unchecked(id) }, 0);
    assert!(!bucket.has(id));
    drop(bucket);
    assert_eq!(array.min_relaxed(), u32::MAX as usize);
    assert_eq!(array.max_relaxed(), 0);
    // #endregion
}

#[test]
fn test_bucket_multithread_read_write() {
    use std::sync::Arc;

    let array: SyncSparseArray<Arc<usize>> = sync_array();
    let sparse: &'static SyncSparseArray<Arc<usize>> = unsafe { std::mem::transmute(&array) };

    let threads: Vec<_> = (100..1250)
        .step_by(30)
        .map(|id| {
            std::thread::spawn(move || {
                let mut bucket = sparse.bucket_lock(id);
                // testing that values propperly dropping after replacing
                let new_arc = Arc::new(id);
                {
                    bucket.set(id, new_arc.clone());
                    let prev_value = bucket.set(id, new_arc.clone());
                    assert_eq!(Arc::strong_count(&new_arc), 3);
                    drop(prev_value)
                }
                // previous inserted arc propperly dropped
                assert_eq!(Arc::strong_count(&new_arc), 2);
            })
        })
        .collect();

    let vec: Arc<Vec<AtomicU64>> = Arc::new((100..1250).step_by(30).map(AtomicU64::new).collect());
    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(
        sparse.min_relaxed(),
        ((100 / BUCKET_DENSITY) * BUCKET_DENSITY)
    );
    assert_eq!(
        sparse.max_relaxed(),
        ((1240 / BUCKET_DENSITY) * BUCKET_DENSITY + BUCKET_DENSITY - 1)
    );

    let threads: Vec<_> = (0usize..(1240f32 / 64f32).ceil() as usize)
        .map(|bucket_id| {
            let vec = vec.clone();
            std::thread::spawn(move || {
                let bucket = unsafe { sparse.bucket(bucket_id * 64) };
                for bit_id in 0..BITS_PER_BUCKET {
                    let bit = bucket.bits() & (1 << bit_id);
                    if bit != 0 {
                        let value =
                            **unsafe { bucket.get_unchecked(bucket_id * BITS_PER_BUCKET + bit_id) }
                                as u64;
                        if let Some(pos) = vec
                            .iter()
                            .position(|array_value| array_value.load(Ordering::Acquire) == value)
                        {
                            vec[pos].store(0, Ordering::Release)
                        }
                    }
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    let expection: Vec<_> = (100..1250).step_by(30).map(|_| 0).collect();
    assert_eq!(
        vec.iter()
            .map(|v| v.load(Ordering::Acquire))
            .collect::<Vec<_>>(),
        expection
    );

    #[allow(dropping_references)]
    drop(sparse);
    drop(array);
}

#[test]
fn test_bucket_multithread_delete() {
    use std::sync::Arc;

    let sparse = Arc::new(sync_array());
    for i in (100..1250).step_by(30) {
        sparse.set_in_place(i, Arc::new(i));
    }

    assert_eq!(
        sparse.min_relaxed(),
        ((100 / BUCKET_DENSITY) * BUCKET_DENSITY)
    );
    assert_eq!(
        sparse.max_relaxed(),
        ((1240 / BUCKET_DENSITY) * BUCKET_DENSITY + BUCKET_DENSITY - 1)
    );

    let threads: Vec<_> = (100..1250)
        .step_by(30)
        .map(|id| {
            let array = sparse.clone();
            std::thread::spawn(move || array.delete_in_place(id))
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    for i in (100..1250).step_by(30) {
        assert_eq!(unsafe { sparse.get_bits_unchecked(i) }, 0);
    }
}

#[test]
#[cfg(not(miri))]
fn sync_sparse_array_min_max() {
    let array = sync_array();
    for id in 1..MAX_ITEMS_PER_ARRAY {
        array.set_in_place(id, id);
        assert_eq!(array.min_relaxed(), 0);
        assert_eq!(
            array.max_relaxed(),
            ((id / 64) * BUCKET_DENSITY + BUCKET_DENSITY - 1)
        );
    }
}

#[test]
#[cfg(not(miri))]
fn sync_sparse_array() {
    assert_eq!(std::mem::size_of::<SyncSparseArray<u128>>(), 1056);

    let array: SyncSparseArray<_> = sync_array();
    let insertion_time = std::time::Instant::now();
    let mut bucket_lock = array.bucket_lock(0);
    let mut current_chunk = 0;
    for id in 1..MAX_ITEMS_PER_ARRAY {
        let next_chunk = get_bucket_index(id);
        if current_chunk != next_chunk {
            drop(bucket_lock);
            bucket_lock = array.bucket_lock(id);
            current_chunk = next_chunk;
        }
        bucket_lock.set(id, id);
    }
    let elapsed = insertion_time.elapsed().as_secs_f32();
    println!("Sync insertion: {}s", elapsed);

    for id in 1..MAX_ITEMS_PER_ARRAY {
        let chunk = unsafe {
            &mut *array
                .buckets
                .get_unchecked(get_bucket_index(id))
                .1
                .load(std::sync::atomic::Ordering::Relaxed)
        };

        assert_eq!(id, unsafe { chunk.slots[get_slot_index(id)].assume_init() });
    }

    let array: SyncSparseArray<_> = sync_array();
    let insertion_time = std::time::Instant::now();
    let mut current_chunk = 0;
    let mut chunk_index = 1;
    let mut chunk_data = Bucket::new(chunk_index as u32);
    for data in 1..MAX_ITEMS_PER_ARRAY {
        let next_chunk = get_bucket_index(data);
        if current_chunk != next_chunk {
            array.buckets.push((
                AtomicU64::new(0),
                AtomicPtr::new(Box::into_raw(Box::new(chunk_data))),
            ));
            chunk_data = Bucket::new(next_chunk as u32);
            chunk_index = 0;
            current_chunk = next_chunk;
        }

        chunk_data.slots[chunk_index] = MaybeUninit::new(data);
        chunk_index += 1;
    }
    array.buckets.push((
        AtomicU64::new(0),
        AtomicPtr::new(Box::into_raw(Box::new(chunk_data))),
    ));
    let elapsed = insertion_time.elapsed().as_secs_f32();
    println!("Direct insertion: {}s", elapsed);

    for id in 1..MAX_ITEMS_PER_ARRAY {
        let chunk = unsafe {
            &mut *array
                .buckets
                .get_unchecked(get_bucket_index(id))
                .1
                .load(std::sync::atomic::Ordering::Relaxed)
        };

        assert_eq!(id, unsafe { chunk.slots[get_slot_index(id)].assume_init() });
    }

    let array: SyncSparseArray<_> = sync_array();
    let insertion_time = std::time::Instant::now();
    for id in 1..MAX_ITEMS_PER_ARRAY {
        array.set_in_place(id, id);
    }
    let elapsed = insertion_time.elapsed().as_secs_f32();
    println!("Set in place: {}s", elapsed);

    for id in 1..MAX_ITEMS_PER_ARRAY {
        let chunk = unsafe {
            &mut *array
                .buckets
                .get_unchecked(get_bucket_index(id))
                .1
                .load(std::sync::atomic::Ordering::Relaxed)
        };

        assert_eq!(id, unsafe { chunk.slots[get_slot_index(id)].assume_init() });
    }
}
