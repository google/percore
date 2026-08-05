// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

//! Linker section based per-core variables
//!
//! The `percore` attribute places a variable's initial value in the `percore` linker section and
//! exposes a `LinkedPerCore` wrapper that accesses the copy for the local CPU. The consuming
//! project must initialize each CPU's percore area and provide its offset by implementing
//! `PercoreLocalOffset` on a type marked with `percore_local_offset`.
//!
//! # Linker script
//!
//! You must include the `percore` section in your linker script, with `__start_percore` and
//! `__stop_percore` symbols to mark its boundaries. E.g.:
//!
//! ```ld
//! percore : ALIGN(CACHE_LINE_SIZE) {
//!     __start_percore = .;
//!     *(SORT_BY_ALIGNMENT(percore percore.*))
//!     /*
//!      * Round the section size up to the actual alignment of the section. This ensures that we
//!      * can have an array of aligned copies of the percore section inside the percore_secondary
//!      * section.
//!      */
//!     . = ALIGN(ALIGNOF(percore));
//!     __stop_percore = .;
//! } >image
//! ```
//!
//! # Initialisation
//!
//! Three possible ways to allocate and initialise each CPU's percore area are:
//!
//! 1. If you know the number of cores at build time, allocate a `percore_secondary` section for it
//!    in your linker script, and provide the `__start_percore_secondary` and
//!    `__stop_percore_secondary` symbols. Copy the appropriate number of copies of the `percore`
//!    section to this section in assembly code before any Rust code runs. On AArch64 bare-metal
//!    targets [`aarch64::percore_copy_secondary_data`] is provided to implement this, and
//!    [`aarch64::percore_calculate_local_offset`] to calculate the area's offset from a CPU linear
//!    index. This must either be done before caches are enabled or with appropriate cache
//!    maintenance operations to ensure that it is visible to all cores.
//! 2. In your Rust entry point before any access to percore variables, copy the appropriate number
//!    of copies of the `percore` section to an appropriately sized area of memory.
//!    [`percore_copy_secondary_data`] is provided to implement this. In this case you must use Rust
//!    synchronisation primitives (e.g. an AtomicBool) to ensure that this happens-before any access
//!    to percore variables.
//! 3. Have each core initialise its own copy of the `percore` section the first time it starts. In
//!    this case there is no distinction between primary and secondary cores, and the original
//!    `percore` section must never be modified (or at least not until all cores have started and
//!    initialised their copies). `percore_copy_secondary_data` can also be used for this.
//!
//! In any case, you must ensure that the alignment of each CPU's percore area is greater than or
//! equal to to the maximum alignment of any percore variable, and that the memory has exposed
//! provenance.
//!
//! # Usage
//!
//! All cores will have their own instance of `VARIABLE` which is initialized to 1.
//!
//! ```
//! # fn exception_free<T>(f: impl FnOnce(percore::ExceptionFree<'_>) -> T) {}
//! use core::cell::RefCell;
//! use percore::{ExceptionLock, derive::percore};
//!
//! #[percore]
//! static VARIABLE: ExceptionLock<RefCell<u64>> = ExceptionLock::new(RefCell::new(1));
//!
//! exception_free(|token| {
//!     assert_eq!(1, *VARIABLE.get().borrow_mut(token));
//!
//!     *VARIABLE.get().borrow_mut(token) = 2;
//!     assert_eq!(2, *VARIABLE.get().borrow_mut(token));
//! });
//! ```

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod aarch64;

use crate::lock::ExceptionLock;
use core::ptr::with_exposed_provenance;
pub use percore_derive::percore;

#[allow(improper_ctypes)]
unsafe extern "Rust" {
    /// Symbol marking the start of the `percore` section.
    #[cfg_attr(
        any(
            target_os = "none",
            target_os = "linux",
            target_os = "android",
            target_os = "fuchsia",
            target_os = "psp",
            target_os = "freebsd",
            target_os = "openbsd",
        ),
        link_name = "__start_percore"
    )]
    #[cfg_attr(
        any(target_os = "macos", target_os = "ios", target_os = "tvos"),
        link_name = "\x01section$start$__DATA$__percore"
    )]
    pub safe static START_PERCORE: ();
    /// Symbol marking the end of the `percore` section.
    #[cfg_attr(
        any(
            target_os = "none",
            target_os = "linux",
            target_os = "android",
            target_os = "fuchsia",
            target_os = "psp",
            target_os = "freebsd",
            target_os = "openbsd",
        ),
        link_name = "__stop_percore"
    )]
    #[cfg_attr(
        any(target_os = "macos", target_os = "ios", target_os = "tvos"),
        link_name = "\x01section$end$__DATA$__percore"
    )]
    pub safe static STOP_PERCORE: ();
}

/// Returns the size in bytes of a single core's `percore` section.
pub fn percore_size() -> usize {
    &raw const STOP_PERCORE as usize - &raw const START_PERCORE as usize
}

/// Duplicates the contents of the initialised percore section into the secondary cores' percore
/// area.
///
/// The function calculates the size of the `percore` section as the difference between the
/// `__start_percore` and `__stop_percore` symbols. Then it copies this memory area into the
/// given `secondary_percore_area` as many times as it fits.
///
/// Panics if the length of `secondary_percore_area` isn't a multiple of the size of the `percore`
/// section.
///
/// This also exposes the provenance of the `secondary_percore_area`, as `LinkedPerCore::get` will
/// later construct a pointer to it with exposed provenance.
///
/// # Safety
///
/// This must only be called before any core accesses any percore variable.
///
/// `secondary_percore_area` must be valid for writes, and must not overlap with the `percore
/// section.
///
/// You must ensure that this initialisation happens-before any percore variables are accessed
/// (according to Rust's memory model). This could for example be achieved by writing to an
/// AtomicBool with release semantics and having other cores wait until they see the written value
/// with acquire semantics.
pub unsafe fn percore_copy_secondary_data(secondary_percore_area: *mut [u8]) {
    let percore_start = (&raw const START_PERCORE).cast::<u8>();
    let percore_size = percore_size();

    assert!(secondary_percore_area.len().is_multiple_of(percore_size));
    let copies = secondary_percore_area.len() / percore_size;

    for i in 0..copies {
        let dest = (secondary_percore_area as *mut u8).wrapping_byte_add(i * percore_size);
        // SAFETY: The caller promises that `secondary_percore_area` is valid to write and doesn't
        // overlap with the `percore` section.
        unsafe {
            percore_start.copy_to_nonoverlapping(dest, percore_size);
        }
    }

    // Expose the provenance of the secondary core area, because `LinkedPerCore::get` will construct
    // a pointer to it with exposed provenance.
    secondary_percore_area.expose_provenance();
}

/// Provides the offset of the local core's percore area.
///
/// The consuming project must implement this trait for a type and mark that type with the
/// [`percore_local_offset!`](crate::percore_local_offset) macro.
///
/// # Safety
///
/// The offset must point to a valid and initialized percore memory area and it must not
/// overflow `isize` for any percore variable and core.
pub unsafe trait PercoreLocalOffset {
    /// Returns the byte offset of the local core's percore area from the `percore` section.
    fn percore_local_offset() -> isize;
}

unsafe extern "Rust" {
    /// Returns the byte offset supplied by the type marked with
    /// [`percore_local_offset`](macro@crate::percore_local_offset).
    ///
    /// The [`PercoreLocalOffset`] safety contract guarantees that the offset is valid for every
    /// per-core variable.
    safe fn percore_local_offset() -> isize;
}

/// A value stored in a linker section containing one copy for each CPU core.
///
/// The initial value (which may also be primary core's value) is stored directly in this wrapper.
/// [`get`](Self::get) adds the local per-core offset to its address to locate the current core's
/// copy.
///
/// This should generally not be constructed directly, but through the [`percore`] macro.
#[repr(transparent)]
pub struct LinkedPerCore<T>(T);

impl<T> LinkedPerCore<T> {
    /// Creates a new instance containing the primary core's value.
    ///
    /// This should generally not be called directly, but through the [`percore`] macro.
    ///
    /// # Safety
    ///
    /// The created variable must be a static placed in the `percore` section and the project must
    /// have a valid `PercoreLocalOffset` implementation. It must only be accessed after
    /// `percore_copy_secondary_data` has run.
    pub const unsafe fn new(value: T) -> Self {
        Self(value)
    }

    /// Returns a shared reference to the value for the current CPU core.
    #[inline(always)]
    pub fn get(&self) -> &T {
        // We need to construct a new pointer with exposed provenance rather than just using
        // `byte_offset` on the pointer to `self.0` because the per-core copy is not part of the
        // same allocation.
        let percore_ptr = with_exposed_provenance::<T>(
            ((&raw const self.0)
                .expose_provenance()
                .cast_signed()
                .wrapping_add(percore_local_offset()))
            .cast_unsigned(),
        );

        debug_assert!(!percore_ptr.is_null());
        debug_assert!(percore_ptr.is_aligned());

        // SAFETY:
        // * The percore region must be aligned to the maximum alignment of any percore variable,
        //   and `&self.0` must be aligned as it comes from a reference, so adding the offset to it
        //   must still be properly aligned. (In debug builds we also double-check with the
        //   debug_assert above.)
        // * The pointer is non-null because the offset is guaranteed to produce a valid address.
        // * The PercoreLocalOffset implementation promises that the calculated pointer points into
        // * the percore memory area which is initialized and it is dereferenceable for the T type.
        // * Aliasing is prevented by each core having its own instance of the variable and by
        //   requiring `ExceptionLock` for `Sync` implementation.
        unsafe { &*percore_ptr }
    }
}

// SAFETY: `LinkedPerCore` is safe between different cores, because each core has its own
// core-local instance of the variable. `ExceptionLock` also prevents concurrent access from runtime
// and exception context.
unsafe impl<T: Send> Sync for LinkedPerCore<ExceptionLock<T>> {}

/// Marks the type that implements [`PercoreLocalOffset`].
///
/// This creates the `percore_local_offset` function used internally by `percore::derive`.
///
/// # Example
///
/// ```
/// use percore::{derive::PercoreLocalOffset, percore_local_offset};
///
/// percore_local_offset!(LocalOffsetImpl);
/// struct LocalOffsetImpl;
///
/// unsafe impl PercoreLocalOffset for LocalOffsetImpl {
///     fn percore_local_offset() -> isize {
///         todo!("Return the appropriate offset for the current core")
///     }
/// }
/// ```
#[macro_export]
macro_rules! percore_local_offset {
    ($t:ident) => {
        #[doc(hidden)]
        #[unsafe(export_name = "percore_local_offset")]
        fn __percore_local_offset() -> isize {
            <$t as $crate::derive::PercoreLocalOffset>::percore_local_offset()
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as percore;
    use crate::ExceptionFree;
    use core::{cell::RefCell, num::NonZero, ptr::NonNull};
    use std::{thread, thread_local};

    // We simulate cores with threads for unit tests, so we use a thread-local for the percore
    // region of each thread.
    thread_local! {
        static PERCORE_REGION: RefCell<Option<NonNull<[u8]>>> = RefCell::new(None);
    }

    percore_local_offset!(PercoreLocalOffsetImpl);
    struct PercoreLocalOffsetImpl;

    // SAFETY: The offset returned always points to a region allocated for the core, initialised
    // with a copy of the percore section.
    unsafe impl PercoreLocalOffset for PercoreLocalOffsetImpl {
        fn percore_local_offset() -> isize {
            PERCORE_REGION.with_borrow_mut(|region| {
                let region = region.get_or_insert_with(|| {
                    let new_region = Box::into_raw(vec![0; percore_size()].into_boxed_slice());
                    // SAFETY: new_region is valid for writes because we just allocated it, and no
                    // percore variables have been accessed on this core yet because this is the
                    // first time percore_local_offset has been called.
                    unsafe {
                        percore_copy_secondary_data(new_region);
                    }
                    NonNull::new(new_region).unwrap()
                });
                // Calculate offset.
                isize::try_from(region.addr().get())
                    .unwrap()
                    .checked_sub((&raw const START_PERCORE).addr().try_into().unwrap())
                    .unwrap()
            })
        }
    }

    #[test]
    fn test_percore_derive() {
        #[percore]
        static VALUE: ExceptionLock<RefCell<u64>> =
            ExceptionLock::new(RefCell::new(0xabcd_ef01_2345_6789));

        // SAFETY: There are no exceptions in the simulated environment of the tests.
        let token = unsafe { ExceptionFree::new() };

        assert_eq!(*VALUE.get().borrow(token).borrow(), 0xabcd_ef01_2345_6789);

        *VALUE.get().borrow_mut(token) = 10;
        assert_eq!(*VALUE.get().borrow(token).borrow(), 10);
    }

    #[test]
    fn derive_unsafe() {
        #[percore]
        static VALUE: ExceptionLock<NonZero<u64>> =
            ExceptionLock::new(unsafe { NonZero::new_unchecked(42) });
    }

    #[test]
    fn multiple_cores() {
        // SAFETY: There are no exceptions in the simulated environment of the tests.
        let token = unsafe { ExceptionFree::new() };

        #[percore]
        static VALUE: ExceptionLock<RefCell<u64>> = ExceptionLock::new(RefCell::new(42));

        assert_eq!(*VALUE.get().borrow_mut(token), 42);
        *VALUE.get().borrow_mut(token) = 1;
        assert_eq!(*VALUE.get().borrow_mut(token), 1);

        thread::spawn(|| {
            // SAFETY: There are no exceptions in the simulated environment of the tests.
            let token = unsafe { ExceptionFree::new() };

            assert_eq!(*VALUE.get().borrow_mut(token), 42);
            *VALUE.get().borrow_mut(token) = 2;
            assert_eq!(*VALUE.get().borrow_mut(token), 2);
        })
        .join()
        .unwrap();

        assert_eq!(*VALUE.get().borrow_mut(token), 1);
    }
}
