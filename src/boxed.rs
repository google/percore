// Copyright 2025 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use crate::{Cores, ExceptionLock, PerCore};
use alloc::boxed::Box;
use core::iter::repeat_with;

// SAFETY: Both different CPU cores and different exception contexts must be treated as separate
// 'threads' for the purposes of Rust's memory model. `PerCore` only allows access to the value for
// the current core, and `ExceptionLock` requires exceptions to be disabled while accessing it which
// prevents concurrent access to its contents from different exception contexts. The combination of
// the two therefore prevents concurrent access to `T`.
unsafe impl<V: Send, C: Cores> Sync for PerCore<Box<[ExceptionLock<V>]>, C> {}

impl<T, C: Cores> PerCore<Box<[T]>, C> {
    /// Gets a shared reference to the value for the current CPU core.
    pub fn get(&self) -> &T {
        &self.values[C::core_index()]
    }
}

impl<T: Default, C: Cores> PerCore<Box<[T]>, C> {
    /// Returns a new `PerCore` wrapping a boxed slice of `core_count` elements, each initialised to
    /// the default value of `T`.
    pub fn new_with_default(core_count: usize) -> Self {
        let boxed_slice = repeat_with(|| Default::default())
            .take(core_count)
            .collect();
        Self::new(boxed_slice)
    }
}
