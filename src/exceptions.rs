// Copyright 2024 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
use aarch64::{mask, restore};

use core::marker::PhantomData;

/// Runs the given function with exceptions masked.
///
/// Only IRQs, FIQs and SErrors can be masked. Synchronous exceptions cannot be masked and so may
/// still occur.
#[cfg(target_arch = "aarch64")]
pub fn exception_free<T>(f: impl FnOnce(ExceptionFree<'_>) -> T) -> T {
    // Mask all exceptions and save previous mask state.
    let prev = mask();
    // SAFETY: We just masked exceptions.
    let token = unsafe { ExceptionFree::new() };

    let result = f(token);

    // SAFETY: `token` has been dropped by now, as its lifetime prevents `f` from storing it.
    unsafe {
        // Restore previous exception mask state.
        restore(prev);
    }

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
