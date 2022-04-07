use ::core::{fmt, mem::align_of, ptr::NonNull};

/// An aligned, non-null, covariant pointer.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MaybeDangling<T>(NonNull<T>);

impl<T> MaybeDangling<T> {
    /// Create a dangling `MaybeDangling<T>`.
    pub const fn dangling() -> Self {
        Self(NonNull::dangling())
    }

    /// Create a `MaybeDangling<T>` from a pointer.
    pub fn new(t: *mut T) -> Option<Self> {
        if t.addr() & align_of::<T>() - 1 != 0 {
            return None;
        }
        NonNull::new(t).map(Self)
    }

    /// Create a `MaybeDangling<T>` from a pointer.
    ///
    /// # Safety
    /// - The pointer must be aligned for `T`.
    /// - The pointer must be non-null.
    pub const unsafe fn new_unchecked(t: *mut T) -> Self {
        // SAFETY: The caller has ensured that the pointer is aligned for `T` and
        // non-null.
        let t = unsafe { NonNull::new_unchecked(t) };
        Self(t)
    }

    /// Cast the pointee from `T` to `U`.
    pub const fn cast<U>(self) -> MaybeDangling<U> {
        MaybeDangling(self.0.cast())
    }

    /// Create a raw pointer to the pointee.
    pub const fn as_ptr(self) -> *mut T {
        self.0.as_ptr()
    }

    /// Create a unique reference to the pointee.
    ///
    /// # Safety
    /// - The pointer must be dereferencable.
    /// - The pointee must be an initialized `T`.
    /// - The resulting reference must be valid during `'a`.
    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        // SAFETY: The pointer is aligned by the invariants of this type. The caller has
        // ensured that the pointer is dereferencable, that the pointee is an
        // initialized `T`, and that the resulting reference is valid during
        // `'a`.
        unsafe { self.0.as_mut() }
    }

    /// Create a shared reference to the pointee.
    ///
    /// # Safety
    /// - The pointer must be dereferencable.
    /// - The pointee must be an initialized `T`.
    /// - The resulting reference must be valid during `'a`.
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        // SAFETY: The pointer is aligned by the invariants of this type. The caller has
        // ensured that the pointer is dereferencable, that the pointee is an
        // initialized `T`, and that the resulting reference is valid during
        // `'a`.
        unsafe { self.0.as_ref() }
    }
}

impl<T> From<&'_ T> for MaybeDangling<T> {
    fn from(t: &'_ T) -> Self {
        // SAFETY: References are trivally aligned and non-null.
        unsafe { Self::new_unchecked(t as *const _ as *mut _) }
    }
}

impl<T> From<&'_ mut T> for MaybeDangling<T> {
    fn from(t: &'_ mut T) -> Self {
        // SAFETY: References are trivally aligned and non-null.
        unsafe { Self::new_unchecked(t as *mut _) }
    }
}

impl<T> fmt::Debug for MaybeDangling<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.0, f)
    }
}

impl<T> fmt::Pointer for MaybeDangling<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.0, f)
    }
}
