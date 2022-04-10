use crate::frame::Idx;
use crate::frame::{ExternalArc, InternalArc, NormalArc};
use crate::machine::L0_FRAME_SIZE;

pub struct InternalPageCap {
    page: InternalArc<()>,
}

pub struct NormalPageCap {
    page: NormalArc<[u8; L0_FRAME_SIZE]>,
}

pub struct ExternalPageCap {
    page: ExternalArc<()>,
}

impl InternalPageCap {
    pub unsafe fn assume_init(frame_number: Idx) -> Option<Self> {
        let page = unsafe { InternalArc::assume_init(frame_number) }?;
        Some(Self { page })
    }

    pub fn into_frame_number(self) -> Idx {
        let Self { page } = self;
        page.into_raw()
    }
}

impl NormalPageCap {
    pub fn new(frame_number: Idx, bytes: [u8; L0_FRAME_SIZE]) -> Option<Self> {
        let page = NormalArc::new(frame_number, bytes)?;
        Some(Self { page })
    }

    pub fn into_frame_number(self) -> Idx {
        let Self { page } = self;
        page.into_raw()
    }
}

impl ExternalPageCap {
    pub unsafe fn assume_init(frame_number: Idx) -> Option<Self> {
        let page = unsafe { ExternalArc::assume_init(frame_number) }?;
        Some(Self { page })
    }

    pub fn into_frame_number(self) -> Idx {
        let Self { page } = self;
        page.into_raw()
    }
}
