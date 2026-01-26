// Copyright 2024 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

//! Safe per-CPU core mutable state on no_std platforms through exception masking.
//!
//! This crate provides two main wrapper types: [`PerCore`] to provide an instance of a value per
//! CPU core, where each core can access only its instance, and [`ExceptionLock`] to guard a value
//! so that it can only be accessed while exceptions are masked. These may be combined with
//! `RefCell` to provide safe per-core mutable state.
//!
//! `ExceptionLock` may also be combined with a spinlock-based mutex (such as one provided by the
//! [`spin`](https://crates.io/crates/spin) crate) to avoid deadlocks when accessing global mutable
//! state from exception handlers.
//!
//! # Example
//!
//! ```
//! use core::cell::RefCell;
//! # #[cfg(target_arch = "aarch64")]
//! use percore::{exception_free, Cores, ExceptionLock, PerCore};
//! # #[cfg(not(target_arch = "aarch64"))]
//! # use percore::{Cores, ExceptionLock, PerCore};
//!
//! /// The total number of CPU cores in the target system.
//! const CORE_COUNT: usize = 2;
//!
//! struct CoresImpl;
//!
//! unsafe impl Cores for CoresImpl {
//!     fn core_index() -> usize {
//!         todo!("Return the index of the current CPU core, 0 or 1")
//!     }
//! }
//!
//! struct CoreState {
//!     // Your per-core mutable state goes here...
//!     foo: u32,
//! }
//!
//! const EMPTY_CORE_STATE: ExceptionLock<RefCell<CoreState>> =
//!     ExceptionLock::new(RefCell::new(CoreState { foo: 0 }));
//! static CORE_STATE: PerCore<[ExceptionLock<RefCell<CoreState>>; CORE_COUNT], CoresImpl> =
//!     PerCore::new([EMPTY_CORE_STATE; CORE_COUNT]);
//!
//! fn main() {
//!     // Mask exceptions while accessing mutable state.
//!     # #[cfg(target_arch = "aarch64")]
//!     exception_free(|token| {
//!         // `token` proves that interrupts are masked, so we can safely access per-core mutable
//!         // state.
//!         CORE_STATE.get().borrow_mut(token).foo = 42;
//!     });
//! }
//! ```

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod boxed;
mod exceptions;
mod lock;

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub use self::exceptions::exception_free;
pub use self::{exceptions::ExceptionFree, lock::ExceptionLock};

use core::marker::PhantomData;

/// Trait abstracting how to get the index of the current CPU core.
///
/// # Safety
///
/// `core_index` must never return the same index on different CPU cores.
pub unsafe trait Cores {
    /// Returns the index of the current CPU core.
    fn core_index() -> usize;
}

/// A type which allows values to be stored per CPU core. Only the value associated with the current
/// CPU core can be accessed.
///
/// To use this type you must first implement the [`Cores`] trait for your platform.
///
/// `C::core_index()` must always return a value less than the length of `V` or there will be a
/// runtime panic.
#[derive(Default)]
#[cfg_attr(
    feature = "zerocopy",
    derive(
        zerocopy::FromBytes,
        zerocopy::Immutable,
        zerocopy::KnownLayout,
        zerocopy::Unaligned
    )
)]
#[repr(transparent)]
pub struct PerCore<V: ?Sized, C: Cores> {
    _cores: PhantomData<C>,
    values: V,
}

impl<V, C: Cores> PerCore<V, C> {
    /// Creates a new set of per-core values.
    pub const fn new(values: V) -> Self {
        Self {
            values,
            _cores: PhantomData,
        }
    }
}

impl<T, C: Cores, const CORE_COUNT: usize> PerCore<[T; CORE_COUNT], C> {
    /// Gets a shared reference to the value for the current CPU core.
    pub fn get(&self) -> &T {
        &self.values[C::core_index()]
    }

    /// Gets a unique reference to the value for the current CPU core.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.values[C::core_index()]
    }
}

// SAFETY: Both different CPU cores and different exception contexts must be treated as separate
// 'threads' for the purposes of Rust's memory model. `PerCore` only allows access to the value for
// the current core, and `ExceptionLock` requires exceptions to be disabled while accessing it which
// prevents concurrent access to its contents from different exception contexts. The combination of
// the two therefore prevents concurrent access to `T`.
unsafe impl<T: Send, C: Cores, const CORE_COUNT: usize> Sync
    for PerCore<[ExceptionLock<T>; CORE_COUNT], C>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    /// A Fake implementation of `Cores` for test, that will always return 0.
    pub struct FakeCoresImpl;

    // SAFETY: Tests are all run on a single core.
    unsafe impl Cores for FakeCoresImpl {
        fn core_index() -> usize {
            0
        }
    }

    #[test]
    fn percore_state() {
        static STATE: PerCore<[ExceptionLock<RefCell<u32>>; 4], FakeCoresImpl> =
            PerCore::new([const { ExceptionLock::new(RefCell::new(42)) }; 4]);

        {
            let token = unsafe { ExceptionFree::new() };
            assert_eq!(*STATE.get().borrow_mut(token), 42);
            *STATE.get().borrow_mut(token) += 1;
            assert_eq!(*STATE.get().borrow_mut(token), 43);
        }
    }
}
