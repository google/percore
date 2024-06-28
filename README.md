# percore

[![crates.io page](https://img.shields.io/crates/v/percore.svg)](https://crates.io/crates/percore)
[![docs.rs page](https://docs.rs/percore/badge.svg)](https://docs.rs/percore)

Safe per-CPU core mutable state on no_std platforms through exception masking.

This crate provides two main wrapper types: `PerCore` to provide an instance of a value per
CPU core, where each core can access only its instance, and `ExceptionLock` to guard a value
so that it can only be accessed while exceptions are masked. These may be combined with
`RefCell` to provide safe per-core mutable state.

`ExceptionLock` may also be combined with a spinlock-based mutex (such as one provided by the
[`spin`](https://crates.io/crates/spin) crate) to avoid deadlocks when accessing global mutable
state from exception handlers.

# Example

```rust
use core::cell::RefCell;
use percore::{exception_free, Cores, ExceptionLock, PerCore};

/// The total number of CPU cores in the target system.
const CORE_COUNT: usize = 2;

struct CoresImpl;

unsafe impl Cores for CoresImpl {
    fn core_index() -> usize {
        todo!("Return the index of the current CPU core, 0 or 1")
    }
}

struct CoreState {
    // Your per-core mutable state goes here...
    foo: u32,
}

const EMPTY_CORE_STATE: ExceptionLock<RefCell<CoreState>> =
    ExceptionLock::new(RefCell::new(CoreState { foo: 0 }));
static CORE_STATE: PerCore<[ExceptionLock<RefCell<CoreState>>; CORE_COUNT], CoresImpl> =
    PerCore::new([EMPTY_CORE_STATE; CORE_COUNT]);

fn main() {
    // Mask exceptions while accessing mutable state.
    exception_free(|token| {
        // `token` proves that interrupts are masked, so we can safely access per-core mutable
        // state.
        CORE_STATE.get().borrow_mut(token).foo = 42;
    });
}
```

This is not an officially supported Google product.

## Supported architectures

Currently only aarch32 and aarch64 are fully supported. The crate will build for other
architectures, but you'll need to provide your own implementation of the `exception_free` function.
Patches are welcome to add support for other architectures.

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
