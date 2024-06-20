// Copyright 2024 The percpu Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use crate::ExceptionFree;
use core::cell::{RefCell, RefMut};

/// Allows access to the given value only while exceptions are masked, allowing it to be shared
/// between exception contexts on a given CPU.
pub struct CriticalCell<T> {
    value: T,
}

impl<T> CriticalCell<T> {
    /// Creates a new CriticalCell containing the given value.
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Gets a unique reference to the contents of the cell, given a token proving that exceptions
    /// are currently masked.
    pub fn borrow<'cs>(&'cs self, _: ExceptionFree<'cs>) -> &'cs T {
        &self.value
    }
}

impl<T> CriticalCell<RefCell<T>> {
    /// Gets a unique reference to the contents of the RefCell, given a token proving that
    /// exceptions are currently masked.
    pub fn borrow_mut<'cs>(&'cs self, token: ExceptionFree<'cs>) -> RefMut<'cs, T> {
        self.borrow(token).borrow_mut()
    }

    /// Returns a raw pointer to the contents of the cell.
    ///
    /// This must not be dereferenced while any `RefMut` to the same cell exists.
    pub fn as_ptr(&self) -> *mut T {
        self.value.as_ptr()
    }
}
