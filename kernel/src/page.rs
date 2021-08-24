use ::core::{cell::UnsafeCell, mem::MaybeUninit};

use crate::frame::Arc;
use crate::frame::Idx;
use crate::machine::L0_FRAME_SIZE;

pub struct L0PageCap {
    page: Arc<Page<[u8; 0x1000]>>,
}

impl L0PageCap {
    pub fn new(frame_number: Idx, bytes: [u8; L0_FRAME_SIZE]) -> Option<Self> {
        let page = Arc::new(frame_number, Page(UnsafeCell::new(MaybeUninit::new(bytes))))?;
        Some(Self { page })
    }

    pub unsafe fn already_init(frame_number: Idx) -> Option<Self> {
        let page = unsafe { Arc::assume_init(frame_number) }?;
        Some(Self { page })
    }

    pub fn into_frame_number(self) -> Idx {
        let Self { page } = self;
        page.into_raw()
    }
}

#[repr(transparent)]
struct Page<T>(UnsafeCell<MaybeUninit<T>>);

// Just a wrapper around `T`, except we never allow references to `T`.
unsafe impl<T> Send for Page<T> where T: Send {}

// We don't allow references to `T`, so this is trivially `Sync`.
unsafe impl<T> Sync for Page<T> where T: Sync {}
