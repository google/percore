// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use super::{__PERCORE_END__, __PERCORE_START__};
use core::arch::naked_asm;

#[allow(improper_ctypes)]
unsafe extern "Rust" {
    /// Symbol marking the start of the `.percore_secondary` section.
    pub safe static __PERCORE_SECONDARY_START__: ();
    /// Symbol marking the end of the `.percore_secondary` section.
    pub safe static __PERCORE_SECONDARY_END__: ();
}

/// Duplicates the contents of the initialised percore section into the secondary cores' percore
/// area. The function is safe to be called from assembly without a stack present. It clobbers
/// registers X0-X6.
///
/// The function calculates the size of the percore section as the difference between the
/// __PERCORE_START__ and __PERCORE_END__ symbols. Then it copies this memory area between the
/// __PERCORE_SECONDARY_START__ and __PERCORE_SECONDARY_END__ symbols as many times as it fits.
/// The copy is done in 16 byte chunks, so these symbols must be aligned to at least a 16 byte
/// boundary. The function is suitable for tiny and small memory models.
///
/// # Safety
///
/// This must only be called before any core accesses any percore variable.
#[unsafe(naked)]
pub unsafe extern "C" fn percore_copy_secondary_data() {
    naked_asm!(
        "bti	c
        adrp	x0, {PERCORE_START}
        add	x0, x0, :lo12:{PERCORE_START}
        adrp	x1, {PERCORE_END}
        add	x1, x1, :lo12:{PERCORE_END}
        adrp	x2, {PERCORE_SECONDARY_START}
        add	x2, x2, :lo12:{PERCORE_SECONDARY_START}
        adrp	x3, {PERCORE_SECONDARY_END}
        add	x3, x3, :lo12:{PERCORE_SECONDARY_END}

        /* Check whether the percore section is empty. */
        cmp	x0, x1
        b.eq	3f

        /* Check whether the percore_secondary section is empty, i.e. no secondary cores */
        cmp	x2, x3
        b.eq	3f

        /* Save source start pointer */
        mov	x4, x0

        /*
         * Per-core loop
         * X0: src
         * X1: src_end
         * X2: dst
         * X3: dst_end (end of the percore area of the last core)
         * X5, X6: data temp
         */

    1:
        mov	x0, x4

        /* Data loop */
    2:
        ldp	x5, x6, [x0], #16
        stp	x5, x6, [x2], #16

        /* src == src_end */
        cmp	x0, x1
        b.ne	2b

        /* dst == dst_end */
        cmp	x2, x3
        b.ne	1b

    3:
        ret",
        PERCORE_START = sym __PERCORE_START__,
        PERCORE_END = sym __PERCORE_END__,
        PERCORE_SECONDARY_START = sym __PERCORE_SECONDARY_START__,
        PERCORE_SECONDARY_END = sym __PERCORE_SECONDARY_END__,
    )
}

/// Calculates the offset of the core's percore area from the percore sections beginning using the
/// following formula: `(__PERCORE_END__ - __PERCORE_START__) * core_index`. The intended use of
/// this function is to use its output to set the offset register of the core. The function is safe
/// to be called from assembly without a stack present. It clobbers registers X0-X2 and returns the
/// offset in X0. The function is suitable for tiny and small memory models.
#[unsafe(naked)]
pub extern "C" fn percore_calculate_local_offset(core_index: usize) -> isize {
    naked_asm!(
        "bti	c
        adrp	x1, {PERCORE_START}
        add	x1, x1, :lo12:{PERCORE_START}
        adrp	x2, {PERCORE_END}
        add	x2, x2, :lo12:{PERCORE_END}
        sub	x1, x2, x1
        mul	x0, x0, x1
        ret",
        PERCORE_START = sym __PERCORE_START__,
        PERCORE_END = sym __PERCORE_END__,
    )
}
