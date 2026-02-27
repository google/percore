// Copyright 2024 The percpu Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use core::arch::asm;

/// Exception mask value which has been saved to later be restored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct Mask(u32);

/// Mask for the SError interrupt mask, IRQ mask and FIQ mask bits of CPSR.
const AIF_MASK: u32 = 0x7 << 6;

impl Mask {
    /// Masks IRQs, FIQs, SErrors and Debug exceptions.
    ///
    /// Returns the previous mask value, to be passed to [`unmask`].
    pub fn mask() -> Self {
        let prev: u32;

        // SAFETY: Writing to this system register doesn't access memory in any way.
        unsafe {
            asm!(
                "mrs {prev}, CPSR",
                "cpsid aif",
                options(nostack),
                prev = out(reg) prev,
            );
        }

        Self(prev & AIF_MASK)
    }

    /// Restores the given previous exception mask value.
    ///
    /// # Safety
    ///
    /// Must not be called while a corresponding `ExceptionFree` token exists.
    pub unsafe fn restore(self) {
        let mask = self.0 | !AIF_MASK;

        // SAFETY: Writing to this system register doesn't access memory in any way. The caller promised
        // that there is no `ExceptionFree` token.
        unsafe {
            asm!(
                "mrs {temp}, CPSR",
                "and {temp}, {temp}, {mask}",
                "msr CPSR, {temp}",
                options(nostack),
                temp = out(reg) _,
                mask = in(reg) mask,
            );
        }
    }
}
