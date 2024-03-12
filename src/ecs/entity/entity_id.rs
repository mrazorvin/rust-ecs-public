use std::{mem::transmute, num::NonZeroU16};

/// <- 16 archetype -> <- 16 index ->
///
/// EntityId(u64::MAX) -> non exist entity
/// EntityId(0)        -> invalid / non exists entity
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(C)]
pub struct EntityId {
    id: NonZeroU16,
    arch: NonZeroU16,
}

impl Default for EntityId {
    fn default() -> Self {
        Self::dead()
    }
}

impl EntityId {
    #[inline]
    pub fn dead() -> Self {
        EntityId {
            arch: unsafe { NonZeroU16::new_unchecked(u16::MAX) },
            id: unsafe { NonZeroU16::new_unchecked(u16::MAX) },
        }
    }

    pub fn id(&self) -> NonZeroU16 {
        self.id
    }

    pub fn idx(&self) -> usize {
        self.id.get() as usize
    }

    pub fn arch(&self) -> NonZeroU16 {
        self.arch
    }

    pub fn archx(&self) -> usize {
        self.arch.get() as usize
    }

    #[inline]
    pub fn as_u32(&self) -> u32 {
        unsafe { transmute(*self) }
    }

    #[inline]
    pub unsafe fn from_u32(raw_entity: u32) -> EntityId {
        transmute(raw_entity)
    }
}

pub fn invalid_new(raw_id: u16) -> EntityId {
    assert!(raw_id != 0);

    EntityId {
        id: unsafe { NonZeroU16::new_unchecked(raw_id) },
        arch: unsafe { NonZeroU16::new_unchecked(u16::MAX) },
    }
}

pub fn new(raw_id: u16, raw_arch: u16) -> EntityId {
    assert!(raw_id != 0 && raw_id != u16::MAX);
    assert!(raw_arch != 0 && raw_arch != u16::MAX);

    EntityId {
        id: unsafe { NonZeroU16::new_unchecked(raw_id) },
        arch: unsafe { NonZeroU16::new_unchecked(raw_arch) },
    }
}

#[test]
fn test_methods() {
    assert_eq!(EntityId::dead().id().get(), u16::MAX);
    assert_eq!(EntityId::dead().arch().get(), u16::MAX);

    assert_eq!(EntityId::dead().as_u32(), u32::MAX);
    assert_eq!(unsafe { EntityId::from_u32(u32::MAX) }, EntityId::dead());

    assert_eq!(
        unsafe { EntityId::from_u32(((u16::MAX as u32) << 16) | 32) },
        EntityId {
            id: unsafe { NonZeroU16::new_unchecked(32) },
            arch: unsafe { NonZeroU16::new_unchecked(u16::MAX) },
        }
    );

    assert_eq!(
        unsafe { EntityId::from_u32(32 << 16 | u16::MAX as u32) },
        EntityId {
            id: unsafe { NonZeroU16::new_unchecked(u16::MAX) },
            arch: unsafe { NonZeroU16::new_unchecked(32) },
        }
    );

    assert_eq!(
        unsafe { EntityId::from_u32(256 << 16 | 128 as u32) },
        EntityId {
            id: unsafe { NonZeroU16::new_unchecked(128) },
            arch: unsafe { NonZeroU16::new_unchecked(256) },
        }
    );
}
