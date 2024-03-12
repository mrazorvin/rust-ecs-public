macro_rules! fetch_component {
  ([_] $ctx:ident $field:ident $type:ident) => {};
  ([&mut $varname:ident] $id:ident $arch:ident $type:ident) => {
    let $varname = unsafe { $arch.get_mut_unchecked($id) };
  };
  ([&$varname:ident] $id:ident $arch:ident $type:ident) => {
    let $varname = unsafe { $arch.get_unchecked($id) };
  };
  ([f!{ $($body:tt)* }] $id:ident $arch:ident $type:ident) => {
    let &$type { $($body)*, .. } = unsafe { $arch.get_unchecked($id) };
  };
  ([fm! { $($body:tt)* }] $id:ident $arch:ident $type:ident) => {
    let &mut $type { $($body)*, .. } = unsafe { &mut $arch.get_mut_unchecked($id) };
  };
}

#[allow(unused_imports)]
pub(crate) use fetch_component;

macro_rules! query_ctx {
  ($params:ident, $($type:ident[$($body:tt)*]),*, |$entity_id:tt| { $($fn:tt)* }) => {{
    paste::paste! {

    let mut archs_iter = crate::ecs::collections::sync_ivec::SameValIter::new();
    $(let mut [<$type:lower _state>] = archs_iter.add(&$type.chunks);)*

    #[allow(unused_parens)]
    while let Some(($([<$type:lower _arch>]),*)) = {
      'next_arch: loop {
        if let Some(mut entry) = archs_iter.next() {
          $(let [<$type:lower _val>] = entry.progress(&mut [<$type:lower _state>]).1.arr;)*
          if entry.complete_is_valid() {
            break 'next_arch Some(($([<$type:lower _val>]),*))
          }
        } else {
          break 'next_arch None;
        }
      }
    } {
        // safety: {arch_id} is guaranteed to exists by code above
        $(let [<$type:lower _arch>] = unsafe { &*[<$type:lower _arch>] };)*

        let mut chunk_iter = crate::ecs::collections::sync_vec::ZipRangeIterator::new();
        $(let mut [<$type:lower _chunk>] = chunk_iter.add(
          &[<$type:lower _arch>].buckets,
           [<$type:lower _arch>].min_relaxed(),
           [<$type:lower _arch>].max_relaxed()
        );)*

        for mut chunk in chunk_iter {
          let entity_offset= chunk.bucket_index() * 64;

          $(let [<$type:lower _chunk>] = chunk.progress(&mut [<$type:lower _chunk>]);)*
          for i in chunk.complete() {
              $(#[allow(unused_mut)] let mut [<$type:lower _bucket>] = unsafe { $type.get_bucket([<$type:lower _arch>], std::mem::transmute(&[<$type:lower _chunk>][i])) };)*
              let value: u64 = $([<$type:lower _bucket>].bits())&*;
              if value == 0 {
                continue;
              }
              for i in 0..64 as usize {
                if (value & (1 << i)) != 0 {
                  let id = entity_offset + i;
                  let $entity_id = id;
                  $(crate::ecs::query::fetch_component!([$($body)*] id [<$type:lower _bucket>] $type);)*
                  $($fn)*
                }
              }
          }
        }
      }
    }}
  };
}

#[allow(unused_imports)]
pub(crate) use query_ctx;

// - cargo expand --tests --lib  query > main_gen.rs
// - cargo miri nextest run --release -j4 --lib -- query
// - cargo test --lib -Zpanic-abort-tests query -- --nocapture
#[test]
fn query_features_test() {
    use std::ops::Deref;

    use crate::ecs::collections::sync_sparse_array::SparseBucketRaw;
    use crate::ecs::collections::sync_sparse_array::{BucketRef, BucketRefMut, SyncSparseArray};
    use crate::ecs::collections::sync_sparse_chunked_store::SyncSparseChunkedStore;

    struct View<T> {
        data: *mut SyncSparseChunkedStore<T>,
    }

    impl<T> Deref for View<T> {
        type Target = SyncSparseChunkedStore<T>;

        fn deref(&self) -> &Self::Target {
            unsafe { &*self.data }
        }
    }

    // #region ### Example - Staic component

    #[derive(Debug)]
    struct Position {
        value: String,
    }

    #[allow(non_snake_case)]
    let Position: View<Position> = {
        let position = Box::new(SyncSparseChunkedStore::new());
        position.set(
            10,
            10,
            Position {
                value: format!("Position {}", 10),
            },
        );
        View {
            data: Box::into_raw(position),
        }
    };

    impl View<Position> {
        pub unsafe fn get_bucket(
            &self,
            sparse_vec: &SyncSparseArray<Position>,
            bucket_ref: &SparseBucketRaw<Position>,
        ) -> BucketRef<Position> {
            unsafe { sparse_vec.bucket_from_ref(bucket_ref) }
        }
    }
    // #endregion

    // #region ### Example - Static component
    #[derive(Debug)]
    struct Attributes {
        value: String,
    }

    #[allow(non_snake_case)]
    let Attributes: View<Attributes> = {
        let attributes = Box::new(SyncSparseChunkedStore::new());
        attributes.set(
            10,
            10,
            Attributes {
                value: format!("Attributes {}", 10),
            },
        );
        View {
            data: Box::into_raw(attributes),
        }
    };

    impl View<Attributes> {
        pub unsafe fn get_bucket(
            &self,
            sparse_vec: &SyncSparseArray<Attributes>,
            bucket_ref: &SparseBucketRaw<Attributes>,
        ) -> BucketRefMut<Attributes> {
            unsafe { sparse_vec.bucket_lock_from_ref(bucket_ref) }
        }
    }
    // #endregion

    // #region ### Example - Dynamic component

    #[derive(Debug)]
    struct Dynamic {
        value: String,
    }

    let dynamic_component: View<Dynamic> = {
        let dynamic = Box::new(SyncSparseChunkedStore::new());
        dynamic.set(
            10,
            10,
            Dynamic {
                value: format!("Dynamic {}", 10),
            },
        );
        View {
            data: Box::into_raw(dynamic),
        }
    };

    impl View<Dynamic> {
        pub unsafe fn get_bucket(
            &self,
            sparse_vec: &SyncSparseArray<Dynamic>,
            bucket_ref: &SparseBucketRaw<Dynamic>,
        ) -> BucketRef<Dynamic> {
            unsafe { sparse_vec.bucket_from_ref(bucket_ref) }
        }
    }
    // #endregion

    // #region ### Example - Test

    struct QueryParams {}
    let _q = QueryParams {};

    let mut result = String::new();

    query_ctx!(
        q,
        Position[f! { ref value }],
        Attributes[&mut attrs],
        dynamic_component[_],
        |_| {
            attrs.value += " + Dynamic variable";
            result = format!("{} {}", value, attrs.value);
        }
    );

    assert_eq!(result, "Position 10 Attributes 10 + Dynamic variable");

    unsafe { drop(Box::from_raw(Position.data)) };
    unsafe { drop(Box::from_raw(Attributes.data)) };
    unsafe { drop(Box::from_raw(dynamic_component.data)) };
    // #endregion
}
