//! Simple synchronization primitives based on spinning that provide internal
//! mutability for objects that are shared across harts.
//!
//! We use a single spinlock for now to protect all shared kernel data
//! structures. Since all the operations the kernel does should be bounded and
//! fairly quick, this shouldn't cause too many issues, and makes reasoning
//! about correctness much easier.
//!
//! To improve performance in the future, we should consider the geometry of the
//! caches and harts into account.
//! we may want to consider alternative lock designs, going fully lock-free, or
//! distributing the state and synchronizing across harts with inter-hart
//! message passing (a multikernel).

use ::core::{
    cell::{Cell, UnsafeCell},
    hint::spin_loop,
    sync::atomic::{
        AtomicU64,
        Ordering::{Acquire, Relaxed, Release},
    },
};

impl Token {
    /// Spin until we can acquire the token.
    pub fn acquire() -> Self {
        let hart_id = HART_ID.get();
        debug_assert_ne!(TOKEN_HOLDER.load(Relaxed), hart_id);
        assert_ne!(hart_id, INVALID_HART_ID);

        loop {
            if !TOKEN_HOLDER.load(Relaxed) != INVALID_HART_ID {
                // If it seems like no one is holding a token, try to acquire it.

                // ORDERING: On success, any future access must happen strictly
                // before any previous access.
                if TOKEN_HOLDER
                    .compare_exchange_weak(INVALID_HART_ID, hart_id, Acquire, Relaxed)
                    .is_ok()
                {
                    return Self(());
                }
            } else {
                spin_loop();
            }
        }
    }

    /// Release the token.
    ///
    /// Just a more explicit `drop`.
    pub fn release(self) {
        drop(self)
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        debug_assert_eq!(TOKEN_HOLDER.load(Relaxed), HART_ID.get());

        // ORDERING: Any previous access must happen strictly before any future
        // access.
        TOKEN_HOLDER.store(INVALID_HART_ID, Release);
    }
}

impl<T> TokenCell<T> {
    /// Construct a new token cell wrapping a `T`.
    pub const fn new(t: T) -> Self {
        Self(UnsafeCell::new(t))
    }

    /// Immutably borrow the contents of the token cell.
    pub fn borrow<'a>(&'a self, _token: &'a Token) -> &'a T {
        // SAFETY: The token is temporally unique, therefore we may borrow the
        // data as long as the token is borrowed.
        unsafe { &*self.0.get() }
    }

    /// Mutably borrow the contents of the token cell.
    pub fn borrow_mut<'a>(&'a self, _token: &'a mut Token) -> &'a mut T {
        // SAFETY: The token is temporally unique, therefore we may mutably
        // borrow the data as long as the token is mutably borrowed.
        unsafe { &mut *self.0.get() }
    }
}

/// A token confers permission to borrow the contents of a token cell.
///
/// A token and its cells are used to separate borrowing permissions from
/// ownership (dropping) permissions. The implementation ensures that there is
/// at most one token at any given time. Together they provide a similar
/// abstraction to that of a [ghost cell][0] (albeit without a brand lifetime
/// since we don't yet need multiple distinct sets of locked objects).
///
/// [0]: https://plv.mpi-sws.org/rustbelt/ghostcell/
#[derive(Debug)]
pub struct Token(());

/// A token cell is a transparent wrapper over a `T` which only allows its
/// contents to be borrowed by the token holder.
///
/// Provides safe, transparent internal mutability.
///
/// [`Token`]: crate::sync::Token
#[repr(transparent)]
pub struct TokenCell<T>(UnsafeCell<T>);

// SAFETY: A token cell is a transparent wrapper over a `T`. The token ensures
// safety when borrowing.
unsafe impl<T> Send for TokenCell<T> where T: Send {}
unsafe impl<T> Sync for TokenCell<T> where T: Sync {}

const INVALID_HART_ID: u64 = u64::MAX;
static TOKEN_HOLDER: AtomicU64 = AtomicU64::new(INVALID_HART_ID);

// The kernel's thread-local variables are really hart-local.
#[thread_local]
static HART_ID: Cell<u64> = Cell::new(INVALID_HART_ID);

// SAFETY: The caller must ensure that the hart ID given is accurate and unique
// to the caller's current hart, and must not be equal to `u64::MAX`.
pub unsafe fn set_hart_id(hart_id: u64) {
    HART_ID.set(hart_id)
}
