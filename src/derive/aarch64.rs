// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

//! Assembly implementations of percore initialisation helper functions for AArch64 bare-metal
//! targets where the number of cores is known at build time.
//!
//! These assume that your linker script has a `percore_secondary` section located immediately
//! after the `percore` section, with `__start_percore_secondary` and `__stop_percore_secondary`
//! symbols marking its start and end. e.g.
//!
//! ```ld
//! percore_secondary (NOLOAD) : ALIGN(ALIGNOF(percore)) {
//!     __start_percore_secondary = .;
//!     . += (__start_percore - __stop_percore) * (CORE_COUNT - 1);
//!     __stop_percore_secondary = .;
//! } >image
//! ```
//!
//! Note that the `percore_secondary` section is only used for secondary cores; the `percore`
//! section itself is used for the primary core's copy of the variables in this case.

use super::{START_PERCORE, STOP_PERCORE};
use core::arch::naked_asm;

#[allow(improper_ctypes)]
unsafe extern "Rust" {
    /// Symbol marking the start of the `percore_secondary` section.
    #[link_name = "__start_percore_secondary"]
    pub safe static START_PERCORE_SECONDARY: ();
    /// Symbol marking the end of the `percore_secondary` section.
    #[link_name = "__stop_percore_secondary"]
    pub safe static STOP_PERCORE_SECONDARY: ();
}

/// Duplicates the contents of the initialised percore section into the secondary cores' percore
/// area. The function is safe to be called from assembly without a stack present. It clobbers
/// registers X0-X6.
///
/// The function calculates the size of the `percore` section as the difference between the
/// `__start_percore` and `__stop_percore` symbols. Then it copies this memory area between the
/// `__start_percore_secondary` and `__stop_percore_secondary` symbols as many times as it fits.
/// The copy is done in 16 byte chunks, so these symbols must be aligned to at least a 16 byte
/// boundary. The function is suitable for tiny and small memory models.
///
/// # Safety
///
/// This must only be called before any core accesses any percore variable.
///
/// You must ensure that this initialisation happens-before any percore variables are accessed
/// (according to Rust's memory model). This could be achieved by calling it from assembly before
/// caches are enabled or any Rust code runs.
#[unsafe(naked)]
pub unsafe extern "C" fn percore_copy_secondary_data() {
    naked_asm!(
        "bti	c
        adrp	x0, {START_PERCORE}
        add	x0, x0, :lo12:{START_PERCORE}
        adrp	x1, {STOP_PERCORE}
        add	x1, x1, :lo12:{STOP_PERCORE}
        adrp	x2, {START_PERCORE_SECONDARY}
        add	x2, x2, :lo12:{START_PERCORE_SECONDARY}
        adrp	x3, {STOP_PERCORE_SECONDARY}
        add	x3, x3, :lo12:{STOP_PERCORE_SECONDARY}

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
        START_PERCORE = sym START_PERCORE,
        STOP_PERCORE = sym STOP_PERCORE,
        START_PERCORE_SECONDARY = sym START_PERCORE_SECONDARY,
        STOP_PERCORE_SECONDARY = sym STOP_PERCORE_SECONDARY,
    )
}

/// Calculates the offset of the core's percore area from the percore sections beginning using the
/// following formula: `(__stop_percore - __stop_percore) * core_index`. The intended use of
/// this function is to use its output to set the offset register of the core. The function is safe
/// to be called from assembly without a stack present. It clobbers registers X0-X2 and returns the
/// offset in X0. The function is suitable for tiny and small memory models.
#[unsafe(naked)]
pub extern "C" fn percore_calculate_local_offset(core_index: usize) -> isize {
    naked_asm!(
        "bti	c
        adrp	x1, {START_PERCORE}
        add	x1, x1, :lo12:{START_PERCORE}
        adrp	x2, {STOP_PERCORE}
        add	x2, x2, :lo12:{STOP_PERCORE}
        sub	x1, x2, x1
        mul	x0, x0, x1
        ret",
        START_PERCORE = sym START_PERCORE,
        STOP_PERCORE = sym STOP_PERCORE,
    )
}
