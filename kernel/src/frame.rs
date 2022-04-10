//! Safe tracking and access to frames of physical memory.
//!
//! We allow user level management of physical memory frames, including those
//! used for dynamic kernel data, but we enforce that this management is safe
//! with atomic reference counting.

use {
    crate::{
        machine::{FRAME_COUNT, L0_FRAME_SIZE},
        ptr::MaybeDangling,
    },
    ::core::{
        any::type_name,
        borrow::Borrow,
        fmt,
        marker::PhantomData,
        mem::forget,
        mem::{align_of, size_of},
        ops::Deref,
        sync::atomic::{
            Ordering::{Acquire, Relaxed, Release},
            {AtomicPtr, AtomicU32, AtomicU8},
        },
    },
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Idx(u32);

impl Idx {
    const MAX: u32 = crate::machine::FRAME_COUNT as u32 - 1;
    const FRAME_COUNT_CHECK: () = assert!(crate::machine::FRAME_COUNT <= u32::MAX as usize);

    pub const fn from_raw(value: usize) -> Option<Self> {
        // Force evaluation of the above static assertion.
        forget(Self::FRAME_COUNT_CHECK);

        if value > Self::MAX as usize {
            return None;
        }
        Some(Self(value as u32))
    }

    pub const fn into_raw(&self) -> usize {
        self.0 as usize
    }
}

impl<T> Arc<T, NormalPolicy> {
    pub fn new(idx: Idx, t: T) -> Option<Self> {
        unsafe {
            Self::new_with(idx, FrameKind::Normal, |frame| {
                frame.as_ptr().write(t);
            })
        }
    }

    fn get(&self) -> &T {
        let (frame_kind, ref_count, frame) = Self::frame(self.idx);
        debug_assert_eq!(frame_kind, FrameKind::Normal);
        debug_assert!(ref_count.load(Relaxed) > 1);
        // SAFETY: There exist no mutable references to this frame because the
        // reference count is at least one. Any shared references to the frame
        // must share our pointee type since they must derive from the same
        // original pointer. The frame's lifetime extends until the pointer is
        // dropped.
        unsafe { frame.as_ref() }
    }
}

impl<T: Copy> Arc<T, InternalPolicy> {
    pub unsafe fn assume_init(idx: Idx) -> Option<Self> {
        unsafe { Self::new_with(idx, FrameKind::Internal, |_frame| {}) }
    }
}

impl<T: Copy> Arc<T, ExternalPolicy> {
    pub unsafe fn assume_init(idx: Idx) -> Option<Self> {
        unsafe { Self::new_with(idx, FrameKind::External, |_frame| {}) }
    }
}

impl<T, Policy: sealed::ArcPolicy> Arc<T, Policy> {
    const SIZE_CHECK: () = assert!(size_of::<T>() <= L0_FRAME_SIZE);
    const ALIGN_CHECK: () = assert!(align_of::<T>() <= L0_FRAME_SIZE);

    unsafe fn new_with(
        idx: Idx,
        expected_frame_kind: FrameKind,
        f: impl FnOnce(MaybeDangling<T>),
    ) -> Option<Self> {
        // Force evaluation of the above static assertions.
        forget(Self::SIZE_CHECK);
        forget(Self::ALIGN_CHECK);

        let (frame_kind, ref_count, frame) = Self::frame(idx);

        if frame_kind != expected_frame_kind {
            return None;
        }

        // ORDERING: Any previous access to the frame must happen strictly
        // before the construction.
        ref_count.compare_exchange(0, 1, Acquire, Relaxed).ok()?;

        // SAFETY: There exist no other references to this frame because the
        // reference count is one. The frame's lifetime extends until the
        // pointer is dropped.
        f(frame);
        // ORDERING: We impose no ordering on loads and stores to the frame
        // itself since the construction, destruction, and any sending of this
        // pointer will impose sufficient ordering.
        ref_count.store(2, Relaxed);
        Some(Self {
            idx,
            _t: PhantomData,
            _policy: PhantomData,
        })
    }

    fn frame(idx: Idx) -> (FrameKind, &'static AtomicU32, MaybeDangling<T>) {
        let ref_count = &REF_COUNTS[idx.into_raw()];
        let frame_kind = FRAME_KINDS[idx.into_raw()]
            .load(Relaxed)
            .try_into()
            .unwrap();
        let frame_mapping_addr = FRAME_MAPPING_ADDR.load(Relaxed);
        assert!(!frame_mapping_addr.is_null());
        let addr = frame_mapping_addr.map_addr(|addr| addr + idx.into_raw() * L0_FRAME_SIZE);
        let ptr = MaybeDangling::new(addr).unwrap();
        (frame_kind, ref_count, ptr.cast())
    }
}

impl<T, Policy: sealed::ArcPolicy> Arc<T, Policy>
// Technically this is possible to use safely even
// without requiring `T: Send + Sync` but that
// would be a massive footgun without good reason.
where
    T: Send + Sync,
{
    pub fn into_raw(self) -> Idx {
        let Self { idx, _t, _policy } = self;
        forget(self);
        idx
    }

    /// # Safety
    /// `idx` must have been returned from a previous call to
    /// `into_raw`.
    pub unsafe fn from_raw(idx: Idx) -> Self {
        Self {
            idx,
            _t: PhantomData,
            _policy: PhantomData,
        }
    }
}

impl<T, Policy: sealed::ArcPolicy> Clone for Arc<T, Policy> {
    fn clone(&self) -> Self {
        let (frame_kind, ref_count, _) = Self::frame(self.idx);
        debug_assert_eq!(frame_kind, FrameKind::Normal);
        debug_assert!(ref_count.load(Relaxed) > 1);
        // ORDERING: We impose no ordering on loads and stores to the frame
        // itself since the construction, destruction, and any sending of this
        // pointer will impose sufficient ordering.
        let ref_count = ref_count.fetch_add(1, Relaxed);
        // We assume that there aren't enough threads to increment this fast enough
        // that we wrap around (which could let another thread racily blow past the
        // initialization check).
        assert!(ref_count < u32::MAX / 2);
        Self {
            idx: self.idx,
            _t: PhantomData,
            _policy: PhantomData,
        }
    }
}

impl<T, Policy: sealed::ArcPolicy> Drop for Arc<T, Policy> {
    fn drop(&mut self) {
        let (frame_kind, ref_count, frame) = Self::frame(self.idx);
        debug_assert_eq!(frame_kind, FrameKind::Normal);
        debug_assert!(ref_count.load(Relaxed) > 1);
        // ORDERING: Any previous access to the frame must happen strictly
        // before the destruction.
        if ref_count.fetch_sub(1, Release) == 1 {
            // ORDERING: Any previous access to the frame must happen strictly
            // before the destruction.
            ref_count.load(Acquire);
            let frame = frame.as_ptr();
            // SAFETY: There exist no references to this frame because the
            // reference count is zero. The frame's lifetime extends until
            // the pointer is dropped.
            unsafe { frame.drop_in_place() };
            // ORDERING: The destruction must happen strictly before any future
            // construction.
            ref_count.store(0, Release);
        }
    }
}

// SAFETY: `FrameArc<T>` can be used to send `T` between threads, so
// `FrameArc<T>: Send` iff `T: Send`.
unsafe impl<T, Policy: sealed::ArcPolicy> Send for Arc<T, Policy> where T: Send {}

// SAFETY: `&FrameArc<T>` can be used to send `T` and `T&` between threads, so
// `FrameArc<T>: Send` iff `T: Send + Sync`.
unsafe impl<T, Policy: sealed::ArcPolicy> Sync for Arc<T, Policy> where T: Send + Sync {}

/// Framed objects are used to store kernel objects referenced by capabilities.
///
/// Many capabilities and kernel objects may refer to other kernel objects,
/// allowing for a multiply-owned graph structure. The user controls allocation
/// and deallocation of frames, and the kernel enforces that said allocation
/// can only use unreferenced frames.
pub struct Arc<T, Policy: sealed::ArcPolicy> {
    idx: Idx,
    _t: PhantomData<T>,
    _policy: PhantomData<Policy>,
}

mod sealed {
    pub trait ArcPolicy {}
}

pub enum InternalPolicy {}
impl sealed::ArcPolicy for InternalPolicy {}

pub enum NormalPolicy {}
impl sealed::ArcPolicy for NormalPolicy {}

pub enum ExternalPolicy {}
impl sealed::ArcPolicy for ExternalPolicy {}

pub type InternalArc<T> = Arc<T, InternalPolicy>;
pub type NormalArc<T> = Arc<T, NormalPolicy>;
pub type ExternalArc<T> = Arc<T, ExternalPolicy>;

// TODO: Allow multiple levels of frames to be allocated.
// TODO: Dynamically allocate this table with the actual valid range of
// physical addresses so we don't bloat the kernel binary with a massive table.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FrameKind {
    Internal,
    Normal,
    External,
}

impl TryFrom<u8> for FrameKind {
    type Error = ();
    fn try_from(val: u8) -> Result<Self, Self::Error> {
        const INTERNAL: u8 = FrameKind::Internal as u8;
        const NORMAL: u8 = FrameKind::Normal as u8;
        const EXTERNAL: u8 = FrameKind::External as u8;
        let frame_kind = match val {
            INTERNAL => FrameKind::Internal,
            NORMAL => FrameKind::Normal,
            EXTERNAL => FrameKind::External,
            _ => return Err(()),
        };
        Ok(frame_kind)
    }
}

static FRAME_KINDS: [AtomicU8; FRAME_COUNT] = {
    const INIT: AtomicU8 = AtomicU8::new(FrameKind::Internal as u8);
    [INIT; FRAME_COUNT]
};
static REF_COUNTS: [AtomicU32; FRAME_COUNT] = {
    const INIT: AtomicU32 = AtomicU32::new(0);
    [INIT; FRAME_COUNT]
};
static FRAME_MAPPING_ADDR: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

pub unsafe fn set_frame_mapping_addr(addr: *mut ()) {
    FRAME_MAPPING_ADDR.store(addr, Relaxed);
}

pub unsafe fn mark_normal(idx: Idx) {
    FRAME_KINDS[idx.into_raw()].store(FrameKind::Normal as u8, Relaxed);
}

pub unsafe fn mark_device(idx: Idx) {
    FRAME_KINDS[idx.into_raw()].store(FrameKind::External as u8, Relaxed);
}

impl<T> AsRef<T> for Arc<T, NormalPolicy> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T> Borrow<T> for Arc<T, NormalPolicy> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T> Deref for Arc<T, NormalPolicy> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> fmt::Display for Arc<T, NormalPolicy>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T> fmt::Debug for Arc<T, NormalPolicy>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(type_name::<Self>())
            .field(self.get())
            .finish()
    }
}

impl<T> fmt::Pointer for Arc<T, NormalPolicy> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}
