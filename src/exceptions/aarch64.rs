// Copyright 2024 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use core::arch::asm;

/// Masks IRQs, FIQs, SErrors and Debug exceptions.
///
/// Returns the previous mask value, to be passed to [`unmask`].
pub fn mask() -> u64 {
    let prev;

    // SAFETY: Writing to this system register doesn't access memory in any way.
    unsafe {
        asm!(
            "mrs {prev:x}, DAIF",
            "msr DAIFSet, #0xf",
            options(nostack),
            prev = out(reg) prev,
        );
    }

    prev
}

/// Restores the given previous exception mask value.
///
/// # Safety
///
/// Must not be called while a corresponding `ExceptionFree` token exists.
pub unsafe fn restore(prev: u64) {
    // SAFETY: Writing to this system register doesn't access memory in any way. The caller promised
    // that there is no `ExceptionFree` token.
    unsafe {
        asm!(
            "msr DAIF, {prev:x}",
            options(nostack),
            prev=in(reg)prev,
        );
    }
}
