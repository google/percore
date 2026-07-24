// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

//! Linker section based per-core variables
//!
//! The `percore` attribute places a variable's initial value in the `.percore` linker section and
//! exposes a `LinkedPerCore` wrapper that accesses the copy for the local CPU. The consuming
//! project must initialize each CPU's percore area and provide its offset by implementing
//! `PercoreLocalOffset` on a type marked with `percore_local_offset`.
//!
//! On AArch64 bare-metal targets, `percore_copy_secondary_data` initializes the secondary percore
//! areas and `percore_calculate_local_offset` calculates an area's offset from a CPU linear index.

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
mod aarch64;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub use aarch64::{percore_calculate_local_offset, percore_copy_secondary_data};

use crate::lock::ExceptionLock;
use core::ptr::NonNull;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
#[allow(improper_ctypes)]
unsafe extern "Rust" {
    /// Symbol marking the start of the percore section.
    static __PERCORE_START__: ();
    /// Symbol marking the end of the percore section.
    static __PERCORE_END__: ();
    /// Symbol marking the start of the secondary cores' percore section.
    static __PERCORE_SECONDARY_START__: ();
    /// Symbol marking the end of the secondary cores' percore section.
    static __PERCORE_SECONDARY_END__: ();
}

/// Provides the offset of the local core's percore area.
///
/// The consuming project must implement this trait for a type and mark that type with
/// `percore_local_offset`.
///
/// # Safety
///
/// The offset must point to a valid and initialized percore memory area and it must not
/// overflow `isize` for any percore variable and core.
pub unsafe trait PercoreLocalOffset {
    /// Returns the byte offset of the local core's percore area from the `.percore` section.
    fn percore_local_offset() -> usize;
}

unsafe extern "Rust" {
    /// Returns the byte offset supplied by the type marked with
    /// [`percore_local_offset`](macro@crate::percore_local_offset).
    ///
    /// The [`PercoreLocalOffset`] safety contract guarantees that the offset is valid for every
    /// per-core variable.
    pub safe fn percore_local_offset() -> usize;
}

/// A value stored in a linker section containing one copy for each CPU.
///
/// The primary CPU's value is stored directly in this wrapper. [`get`](Self::get) adds the local
/// per-core offset to its address to locate the current CPU's copy.
#[repr(transparent)]
pub struct LinkedPerCore<T>(T);

impl<T> LinkedPerCore<T> {
    /// Creates a new instance containing the primary CPU's value.
    ///
    /// # Safety
    ///
    /// The created variable must be a static placed in the `.percore` section and the project must
    /// have a valid `PercoreLocalOffset` implementation.
    pub const unsafe fn new(value: T) -> Self {
        Self(value)
    }

    /// Returns a shared reference to the value of the current CPU.
    #[inline(always)]
    pub fn get(&self) -> &T {
        // Safety: PercoreLocalOffset guarantees a valid offset.
        let percore_ptr = unsafe { NonNull::from_ref(&self.0).byte_add(percore_local_offset()) };

        // Safety:
        // * Alignment: TODO: we need something like const{assert!(core::mem::align_of::<T>() <= CACHE_WRITEBACK_SIZE)}
        // * The pointer is non-null because it is constructed from NonNull and the offset produces
        //   a valid address.
        // * The PercoreLocalOffset implementation promises that the calculated pointer points into
        // * the percore memory area which is initialized and it is dereferenceable for the T type.
        // * Aliasing is prevented by each core having its own instance of the variable and by
        //   requiring `ExceptionLock` for `Sync` implementation.
        unsafe { percore_ptr.as_ref() }
    }
}

// Safety: `LinkedPerCore` is safe between different cores, because each core has its own
// core-local instance of the variable. `ExceptionLock` also prevents concurrent access from runtime
// and exception context.
unsafe impl<T: Send> Sync for LinkedPerCore<ExceptionLock<T>> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as percore;
    use crate::ExceptionFree;
    use core::cell::RefCell;

    #[percore::percore]
    static VALUE: ExceptionLock<RefCell<u64>> =
        ExceptionLock::new(RefCell::new(0xabcd_ef01_2345_6789));

    #[percore::percore_local_offset]
    struct PercoreLocalOffsetImpl;

    // Safety: Tests use the initialized primary-core value at offset zero.
    unsafe impl PercoreLocalOffset for PercoreLocalOffsetImpl {
        fn percore_local_offset() -> usize {
            0
        }
    }

    #[test]
    fn test_percore_derive() {
        let token = unsafe { ExceptionFree::new() };

        assert_eq!(0xabcd_ef01_2345_6789, *VALUE.get().borrow(token).borrow());

        *VALUE.get().borrow_mut(token) = 10;
        assert_eq!(10, *VALUE.get().borrow(token).borrow());
    }
}
