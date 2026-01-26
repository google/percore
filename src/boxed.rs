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

    /// Gets a unique reference to the value for the current CPU core.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.values[C::core_index()]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExceptionFree, tests::FakeCoresImpl};
    use alloc::boxed::Box;
    use core::{cell::RefCell, iter::repeat_with};
    use spin::{Lazy, once::Once};

    #[test]
    fn percore_boxed_slice() {
        static STATE: Once<PerCore<Box<[ExceptionLock<RefCell<u32>>]>, FakeCoresImpl>> =
            Once::new();

        STATE.call_once(|| {
            let boxed_slice: Box<[ExceptionLock<RefCell<u32>>]> =
                repeat_with(|| ExceptionLock::new(RefCell::new(42)))
                    .take(4)
                    .collect();

            PerCore::<Box<[_]>, _>::new(boxed_slice)
        });

        {
            let token = unsafe { ExceptionFree::new() };
            assert_eq!(*STATE.get().unwrap().get().borrow_mut(token), 42);
            *STATE.get().unwrap().get().borrow_mut(token) += 1;
            assert_eq!(*STATE.get().unwrap().get().borrow_mut(token), 43);
        }
    }

    #[test]
    fn percore_boxed_slice_default() {
        static STATE: Lazy<PerCore<Box<[ExceptionLock<RefCell<u32>>]>, FakeCoresImpl>> =
            Lazy::new(|| PerCore::new_with_default(4));

        {
            let token = unsafe { ExceptionFree::new() };
            assert_eq!(*STATE.get().borrow_mut(token), 0);
            *STATE.get().borrow_mut(token) += 1;
            assert_eq!(*STATE.get().borrow_mut(token), 1);
        }
    }
}
