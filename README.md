# percpu

[![crates.io page](https://img.shields.io/crates/v/percpu.svg)](https://crates.io/crates/percpu)
[![docs.rs page](https://docs.rs/percpu/badge.svg)](https://docs.rs/percpu)

Safe per-CPU mutable state on no_std platforms through exception masking.

This crate provides two main wrapper types: `PerCpu` to provide an instance of a value per
CPU core, where each core can access only its instance, and `ExceptionLock` to guard a value
so that it can only be accessed while exceptions are masked. These may be combined with
`RefCell` to provide safe per-CPU mutable state.

`ExceptionLock` may also be combined with a spinlock-based mutex (such as one provided by the
[`spin`](https://crates.io/crates/spin) crate) to avoid deadlocks when accessing global mutable
state from exception handlers.

# Example

```rust
use core::cell::RefCell;
use percpu::{exception_free, ExceptionLock, PerCpu, Platform};

/// The total number of CPU cores in the target system.
const CORE_COUNT: usize = 2;

struct PlatformImpl;

unsafe impl Platform for PlatformImpl {
    fn core_index() -> usize {
        todo!("Return the index of the current CPU core, 0 or 1")
    }
}

struct CpuState {
    // Your per-CPU mutable state goes here...
    foo: u32,
}

const EMPTY_CPU_STATE: ExceptionLock<RefCell<CpuState>> =
    ExceptionLock::new(RefCell::new(CpuState { foo: 0 }));
static CPU_STATE: PerCpu<ExceptionLock<RefCell<CpuState>>, PlatformImpl, CORE_COUNT> =
    PerCpu::new([EMPTY_CPU_STATE; CORE_COUNT]);

fn main() {
    // Mask exceptions while accessing mutable state.
    exception_free(|token| {
        // `token` proves that interrupts are masked, so we can safely access per-CPU mutable
        // state.
        CPU_STATE.get().borrow_mut(token).foo = 42;
    });
}
```

This is not an officially supported Google product.

## Supported architectures

Currently only aarch64 is fully supported. The crate will build for other architecture, but you'll
need to provide your own implementation of the `exception_free` function. Patches are welcome to add
support for other architectures.

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

If you want to contribute to the project, see details of
[how we accept contributions](CONTRIBUTING.md).
