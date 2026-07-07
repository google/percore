// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
mod aarch64;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub use aarch64::{percore_calculate_local_offset, percore_copy_secondary_data};

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
#[allow(improper_ctypes)]
unsafe extern "C" {
    /// Symbol marking the start of the percore section.
    static __PERCORE_START__: ();
    /// Symbol marking the end of the percore section.
    static __PERCORE_END__: ();
    /// Symbol marking the start of the secondary cores' percore section.
    static __PERCORE_SECONDARY_START__: ();
    /// Symbol marking the end of the secondary cores' percore section.
    static __PERCORE_SECONDARY_END__: ();
}

unsafe extern "Rust" {
    /// The consuming project must define the function in a way that it returns the offset of the
    /// local cores percore area in bytes from the beginning of the percore section. This is done
    /// by marking a function with the `#[percore::percore_local_offset]` attribute.
    pub fn percore_local_offset() -> usize;
}

#[cfg(test)]
mod tests {
    use crate as percore;

    #[percore::percore]
    static VALUE: u64 = 0xabcd_ef01_2345_6789;

    #[percore::percore_local_offset]
    pub fn percore_local_offset_hook() -> usize {
        0
    }

    #[test]
    fn test_percore_derive() {
        assert_eq!(0xabcd_ef01_2345_6789, *VALUE.get());

        assert_eq!(0xabcd_ef01_2345_6789, unsafe { PERCORE_BASE_VALUE });
    }
}
