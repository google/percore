// Copyright 2024 The percpu Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

#![no_std]

mod exceptions;
mod lock;

pub use self::{
    exceptions::{exception_free, ExceptionFree},
    lock::ExceptionLock,
};

use core::marker::PhantomData;

/// Trait abstracting how to get the index of the current CPU core.
///
/// # Safety
///
/// `core_index` must never return the same index on different CPU cores.
pub unsafe trait Platform {
    /// Returns the index of the current CPU core.
    fn core_index() -> usize;
}

/// A type which allows values to be stored per CPU core. Only the value associated with the current
/// CPU can be accessed.
///
/// `P::core_index()` must always return a value <= `CORE_COUNT` or there will be a runtime panic.
pub struct PerCpu<T, P: Platform, const CORE_COUNT: usize> {
    values: [T; CORE_COUNT],
    _platform: PhantomData<P>,
}

impl<T, P: Platform, const CORE_COUNT: usize> PerCpu<T, P, CORE_COUNT> {
    /// Creates a new set of per-CPU values.
    pub const fn new(values: [T; CORE_COUNT]) -> Self {
        Self {
            values,
            _platform: PhantomData,
        }
    }

    /// Gets a shared reference to the value for the current CPU.
    pub fn get(&self) -> &T {
        &self.values[P::core_index()]
    }
}

// SAFETY: Both different CPUs and different exception contexts must be treated as separate
// 'threads' for the purposes of Rust's memory model. `PerCpu` only allows access to the value for
// the current CPU, and `ExceptionLock` requires exceptions to be disabled while accessing it which
// prevents concurrent access to its contents from different exception contexts. The combination of
// the two therefore prevents concurrent access to `T`.
unsafe impl<T: Send, P: Platform, const CORE_COUNT: usize> Sync
    for PerCpu<ExceptionLock<T>, P, CORE_COUNT>
{
}
