// Copyright 2024 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
use aarch64::{mask, restore};

#[cfg(target_arch = "arm")]
mod aarch32;
#[cfg(target_arch = "arm")]
use aarch32::{mask, restore};

use core::marker::PhantomData;

/// Scope guard for exception-free sections.
///
/// This restores the previous mask state when it is dropped, even if something panics in the
/// meantime (if panics are unwound).
///
/// We don't expose this in the crate API because if scope guards are dropped in the wrong order
/// then the mask state won't be properly restored.
#[cfg(target_arch = "aarch64")]
struct ExceptionGuard {
    /// Previous exception mask state.
    prev: u64,
}

#[cfg(target_arch = "aarch64")]
impl ExceptionGuard {
    /// Masks exceptions and return a scope guard which will unmask them when it is dropped.
    ///
    /// # Safety
    ///
    /// The returned `ExceptionGuard` must not be dropped before the token lifetime `'cs` ends. If
    /// multiple `ExceptionGuard`s are created then they must be dropped in the reverse order that
    /// they are created.
    unsafe fn mask<'cs>() -> (Self, ExceptionFree<'cs>) {
        let guard = Self { prev: mask() };
        // SAFETY: We just masked exceptions, and our caller promises not to drop the guard before
        // the token.
        let token = unsafe { ExceptionFree::new() };
        (guard, token)
    }
}

#[cfg(target_arch = "aarch64")]
impl Drop for ExceptionGuard {
    fn drop(&mut self) {
        // SAFETY: When the `ExceptionGuard` was created the caller promised not to drop it before
        // the corresponding token.
        unsafe {
            // Restore previous exception mask state.
            restore(self.prev);
        }
    }
}

/// Runs the given function with exceptions masked.
///
/// Only IRQs, FIQs and SErrors can be masked. Synchronous exceptions cannot be masked and so may
/// still occur.
#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub fn exception_free<T>(f: impl FnOnce(ExceptionFree<'_>) -> T) -> T {
    // Mask all exceptions and save previous mask state.
    // SAFETY: We drop the scope guard after the lifetime of the token ends. Any other
    // `ExceptionGuard`s created within `f` will be dropped before `f` returns, ensuring that the
    // drop order is the reverse of the creation order as required.
    let (scope_guard, token) = unsafe { ExceptionGuard::mask() };

    let result = f(token);

    // `token` has been dropped by now, as its lifetime prevents `f` from storing it.
    drop(scope_guard);

    result
}

/// A token proving that exceptions are currently masked.
///
/// Note that synchronous exceptions cannot be masked and so may still occur.
#[derive(Clone, Copy, Debug)]
pub struct ExceptionFree<'cs> {
    _private: PhantomData<&'cs ()>,
}

impl<'cs> ExceptionFree<'cs> {
    /// Constructs a new instance of `ExceptionFree`, promising that exceptions will remain masked
    /// for at least its lifetime.
    ///
    /// This usually should not be called directly; instead use [`exception_free`].
    ///
    /// # Safety
    ///
    /// `ExceptionFree` must only be constructed while exceptions are masked, and they must not be
    /// unmasked until after it is dropped.
    pub unsafe fn new() -> Self {
        Self {
            _private: PhantomData,
        }
    }
}
