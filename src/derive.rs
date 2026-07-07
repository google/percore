// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

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
/// The offset must point point to a valid and initialized percore memory area and it must not
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

/// Per-core wrapper for `T`.
pub struct PerCoreWrapper<T>(NonNull<T>);

impl<T> PerCoreWrapper<T> {
    /// Creates new instance.
    pub const fn new(value: &T) -> Self {
        Self(NonNull::from_ref(value))
    }

    /// Returns a shared reference to the value of the current CPU.
    #[inline(always)]
    pub fn get(&self) -> &T {
        // Safety: PercoreLocalOffset guarantees a valid offset.
        let percore_ptr = unsafe { self.0.byte_add(percore_local_offset()) };

        // Safety:
        // * Alignment: TODO: we need something like const{assert!(core::mem::align_of::<T>() <= CACHE_WRITEBACK_SIZE)}
        // * The pointer is non-null because it is constructed from NonNull and the offset produces
        //   a valid address.
        // * The PercoreLocalOffset implementation promises that the calculated pointer points into
        // * the percore memory area which is initialized and it is dereferenceable for the T type.
        // * Aliasing is prevented by each core having it's own instance of the variable and by
        //   requiring `ExceptionLock` for `Sync` implementation.
        unsafe { percore_ptr.as_ref() }
    }
}

// Safety: `PerCoreWrapper` is safe between different cores, because each core has its own
// core-local instance of the variable. `ExceptionLock` also prevents concurrent access from runtime
// and exception context.
unsafe impl<T: Send> Sync for PerCoreWrapper<ExceptionLock<T>> {}

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
