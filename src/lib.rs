// Copyright 2024 The percpu Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

//! Safe per-CPU mutable state on no_std platforms through exception masking.
//!
//! This crate provides two main wrapper types: [`PerCpu`] to provide an instance of a value per
//! CPU core, where each core can access only its instance, and [`ExceptionLock`] to guard a value
//! so that it can only be accessed while exceptions are masked. These may be combined with
//! `RefCell` to provide safe per-CPU mutable state.
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
//! use percpu::{exception_free, ExceptionLock, PerCpu, Platform};
//! # #[cfg(not(target_arch = "aarch64"))]
//! # use percpu::{ExceptionLock, PerCpu, Platform};
//!
//! /// The total number of CPU cores in the target system.
//! const CORE_COUNT: usize = 2;
//!
//! struct PlatformImpl;
//!
//! unsafe impl Platform for PlatformImpl {
//!     fn core_index() -> usize {
//!         todo!("Return the index of the current CPU core, 0 or 1")
//!     }
//! }
//!
//! struct CpuState {
//!     // Your per-CPU mutable state goes here...
//!     foo: u32,
//! }
//!
//! const EMPTY_CPU_STATE: ExceptionLock<RefCell<CpuState>> =
//!     ExceptionLock::new(RefCell::new(CpuState { foo: 0 }));
//! static CPU_STATE: PerCpu<ExceptionLock<RefCell<CpuState>>, PlatformImpl, CORE_COUNT> =
//!     PerCpu::new([EMPTY_CPU_STATE; CORE_COUNT]);
//!
//! fn main() {
//!     // Mask exceptions while accessing mutable state.
//!     # #[cfg(target_arch = "aarch64")]
//!     exception_free(|token| {
//!         // `token` proves that interrupts are masked, so we can safely access per-CPU mutable
//!         // state.
//!         CPU_STATE.get().borrow_mut(token).foo = 42;
//!     });
//! }
//! ```

#![no_std]

mod exceptions;
mod lock;

#[cfg(target_arch = "aarch64")]
pub use self::exceptions::exception_free;
pub use self::{exceptions::ExceptionFree, lock::ExceptionLock};

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
/// To use this type you must first implement the [`Platform`] trait for your platform.
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
