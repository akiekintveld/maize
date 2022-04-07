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
        fmt::{Debug, Display, Formatter, Pointer, Result},
        marker::PhantomData,
        mem::forget,
        mem::MaybeUninit,
        mem::{align_of, size_of},
        ops::Deref,
        sync::atomic::{
            Ordering::{Acquire, Relaxed, Release},
            {AtomicPtr, AtomicU32},
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

impl<T> Arc<T> {
    const SIZE_CHECK: () = assert!(size_of::<T>() <= L0_FRAME_SIZE);
    const ALIGN_CHECK: () = assert!(align_of::<T>() <= L0_FRAME_SIZE);

    pub fn new(idx: Idx, t: T) -> Option<Self> {
        // Force evaluation of the above static assertions.
        forget(Self::SIZE_CHECK);
        forget(Self::ALIGN_CHECK);

        let (ref_count, frame) = Self::frame(idx);

        // ORDERING: Any previous access to the frame must happen strictly
        // before the construction.
        ref_count
            .compare_exchange(UNUSED, UNREFERENCED, Acquire, Relaxed)
            .ok()?;

        // SAFETY: There exist no other references to this frame because the
        // reference count is one. The frame's lifetime extends until the
        // pointer is dropped.
        let frame: &mut MaybeUninit<T> = unsafe { frame.cast().as_mut() };
        frame.write(t);
        // ORDERING: We impose no ordering on loads and stores to the frame
        // itself since the construction, destruction, and any sending of this
        // pointer will impose sufficient ordering.
        ref_count.store(1, Relaxed);
        Some(Self {
            idx,
            _t: PhantomData,
        })
    }

    pub unsafe fn assume_init(idx: Idx) -> Option<Self> {
        // Force evaluation of the above static assertions.
        forget(Self::SIZE_CHECK);
        forget(Self::ALIGN_CHECK);

        let (ref_count, frame) = Self::frame(idx);

        // ORDERING: Any previous access to the frame must happen strictly
        // before the construction.
        ref_count
            .compare_exchange(UNREFERENCED, 1, Acquire, Relaxed)
            .ok()?;

        // SAFETY: There exist no other references to this frame because the
        // reference count is one. The frame's lifetime extends until the
        // pointer is dropped.
        let frame: &mut MaybeUninit<T> = unsafe { frame.cast().as_mut() };
        // SAFETY: The caller has ensured that this memory contains a valid
        // object.
        unsafe { frame.assume_init_mut() };
        // ORDERING: We impose no ordering on loads and stores to the frame
        // itself since the construction, destruction, and any sending of this
        // pointer will impose sufficient ordering.
        ref_count.store(1, Relaxed);
        Some(Self {
            idx,
            _t: PhantomData,
        })
    }

    fn get(&self) -> &T {
        let (ref_count, frame) = Self::frame(self.idx);
        debug_assert_ne!(ref_count.load(Relaxed), UNUSED);
        debug_assert_ne!(ref_count.load(Relaxed), UNREFERENCED);
        // SAFETY: There exist no mutable references to this frame because the
        // reference count is at least one. Any shared references to the frame
        // must share our pointee type since they must derive from the same
        // original pointer. The frame's lifetime extends until the pointer is
        // dropped.
        unsafe { frame.as_ref() }
    }

    fn frame(idx: Idx) -> (&'static AtomicU32, MaybeDangling<T>) {
        let ref_count = &REF_COUNTS[idx.into_raw()];
        let frame_mapping_addr = FRAME_MAPPING_ADDR.load(Relaxed);
        assert!(!frame_mapping_addr.is_null());
        let addr = frame_mapping_addr.map_addr(|addr| { addr + idx.into_raw() * L0_FRAME_SIZE});
        let ptr = MaybeDangling::new(addr).unwrap();
        (ref_count, ptr.cast())
    }
}

impl<T> Arc<T>
// Technically this is possible to use safely even without requiring
// `T: Send + Sync` but that would be a massive footgun without good reason.
where
    T: Send + Sync,
{
    pub fn into_raw(self) -> Idx {
        let Self { idx, _t } = self;
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
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let (ref_count, _) = Self::frame(self.idx);
        // ORDERING: We impose no ordering on loads and stores to the frame
        // itself since the construction, destruction, and any sending of this
        // pointer will impose sufficient ordering.
        let ref_count = ref_count.fetch_add(1, Relaxed);
        assert_ne!(ref_count, REF_COUNT_MAX);
        debug_assert_ne!(ref_count, UNUSED);
        debug_assert_ne!(ref_count, UNREFERENCED);
        Self {
            idx: self.idx,
            _t: PhantomData,
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        let (ref_count, frame) = Self::frame(self.idx);
        debug_assert_ne!(ref_count.load(Relaxed), 0);
        // ORDERING: Any previous access to the frame must happen strictly
        // before the destruction.
        if ref_count.fetch_sub(1, Release) == UNREFERENCED {
            // ORDERING: Any previous access to the frame must happen strictly
            // before the destruction.
            ref_count.load(Acquire);
            // SAFETY: There exist no references to this frame because the
            // reference count is zero. The frame's lifetime extends until
            // the pointer is dropped.
            let frame = frame.as_ptr();
            unsafe { frame.drop_in_place() };
            // ORDERING: The destruction must happen strictly before any future
            // construction.
            ref_count.store(UNUSED, Release);
        }
    }
}

// SAFETY: `FrameArc<T>` can be used to send `T` between threads, so
// `FrameArc<T>: Send` iff `T: Send`.
unsafe impl<T> Send for Arc<T> where T: Send {}

// SAFETY: `&FrameArc<T>` can be used to send `T` and `T&` between threads, so
// `FrameArc<T>: Send` iff `T: Send + Sync`.
unsafe impl<T> Sync for Arc<T> where T: Send + Sync {}

/// Framed objects are used to store kernel objects referenced by capabilities.
///
/// Many capabilities and kernel objects may refer to other kernel objects,
/// allowing for a multiply-owned graph structure. The user controls allocation
/// and deallocation of frames, and the kernel enforces that said allocation
/// can only use unreferenced frames.
pub struct Arc<T> {
    idx: Idx,
    _t: PhantomData<T>,
}

// Mark all of the physical frames as initially having existing references,
// so we don't accidentally trample on things before we know what memory is
// ours to play with.

// TODO: Allow multiple levels of frames to be allocated.
// TODO: Dynamically allocate this table with the actual valid range of
// physical addresses so we don't bloat the kernel binary with a massive table.
const UNREFERENCED: u32 = 0;
const UNUSED: u32 = u32::MAX;
const REF_COUNT_MAX: u32 = u32::MAX - 1;
pub const FREE_FRAMES_START: usize = 0xc_0000;
static REF_COUNTS: [AtomicU32; FRAME_COUNT] = {
    const REF_COUNT_INIT: AtomicU32 = AtomicU32::new(UNUSED);
    let mut ref_counts = [REF_COUNT_INIT; FRAME_COUNT];
    let mut frame_number = 0x0;
    while frame_number < FREE_FRAMES_START {
        ref_counts[frame_number] = AtomicU32::new(UNREFERENCED);
        frame_number += 1;
    }
    ref_counts
};
static FRAME_MAPPING_ADDR: AtomicPtr<[u8; FRAME_COUNT]> = AtomicPtr::new(core::ptr::null_mut());

pub fn set_frame_mapping_addr(addr: *mut ()) {
    FRAME_MAPPING_ADDR.store(addr.cast(), Relaxed);
}

impl<T> AsRef<T> for Arc<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T> Borrow<T> for Arc<T> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> Display for Arc<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.get().fmt(f)
    }
}

impl<T> Debug for Arc<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple(type_name::<Self>())
            .field(self.get())
            .finish()
    }
}

impl<T> Pointer for Arc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.get().fmt(f)
    }
}
