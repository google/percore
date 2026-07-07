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

## Example

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

Some more complete examples are provided in the [examples/aarch64_qemu](examples/aarch64_qemu/)
directory.

This is not an officially supported Google product.

## Supported architectures

Currently only aarch32 and aarch64 are fully supported. The crate will build for other
architectures, but you'll need to provide your own implementation of the `exception_free` function.
Patches are welcome to add support for other architectures.

# Derive

The `derive` feature enables the use of the `#[percore::percore]` attribute which changes global
variables into per-core variables. This is an orthogonal way of declaring per-core variables to the
regular `PerCore` based method.

By default, the primary core is assumed to have index 0, with the secondary cores following it
sequentially. Each core has its own per-core area, and these areas are laid out contiguously in
memory, like an array of sections. Only the primary core's area, represented by the `.percore`
section is included in the image, and it contains the initial values for per-core variables. The
corresponding areas for the secondary cores, represented by the `.percore_secondary` section, are
not included in the image. The provided functions are suitable for tiny and small memory models.

Projects may use a different memory layout, provided that they copy the initial variable values into
each secondary core's per-core area and implement `percore::derive::PercoreLocalOffset` on a type
marked with `#[percore::percore_local_offset]` to retrieve the appropriate offset.

Pros:
* At the declaration of the variable it is not necessary to have a `Cores` implementation.
* Each core's per-core variables are packed together which results in more efficient cache
  utilization.
* Better performance.

Cons:
* Can only be used for global variables.
* More complex setup.
* Currently, this is only implemented for `AArch64` targets.

It requires the following actions from the consuming project:

* Call `percore::derive::percore_copy_secondary_data()` on the primary core at boot time.
* Calculate the local core offset at boot time on each core using
  `percore_calculate_local_offset(core_index)`.
* Store this offset in a project specific way.
* Implement `percore::derive::PercoreLocalOffset` for a type that retrieves the previously stored
  offset and mark the type with `#[percore::percore_local_offset]`.
* Allocate `.percore` and `.percore_secondary` sections in the linker script and mark their
  boundaries using the `__PERCORE_START__`, `__PERCORE_END__`, `__PERCORE_SECONDARY_START__` and
  `__PERCORE_SECONDARY_END__` symbols. It is recommended to align these sections to the cache line
  size but at least to 16 bytes on `AArch64`.

## Example

### Initialization

The primary core initializes the secondary per-core areas. Each secondary core then calculates its
offset from its linear index and stores it in `TPIDR_EL1` before entering `main`.

```rust
use core::arch::global_asm;

global_asm!(
    "init_primary:
        bl {percore_copy_secondary_data}

    init_secondary:
        bl calculate_core_index
        /* X0 now contains the local core's linear index */

        /* Calculate and store local offset */
        bl {percore_calculate_local_offset}
        msr tpidr_el1, x0

        b main
    ",
    percore_copy_secondary_data = sym percore_copy_secondary_data,
    percore_calculate_local_offset = sym percore_calculate_local_offset,
);
```

### `PercoreLocalOffsetImpl` implementation

This implementation returns the offset from `TPIDR_EL1`, the EL1 Software Thread ID Register.

```rust
use percore::derive::PercoreLocalOffset;

#[percore::percore_local_offset]
struct PercoreLocalOffsetImpl;

// Safety: Each core initializes TPIDR_EL1 with the offset of its valid percore area before
// accessing any percore variable.
unsafe impl PercoreLocalOffset for PercoreLocalOffsetImpl {
    fn percore_local_offset() -> usize {
        read_tpidr_el1()
    }
}
```

### Linker script

The linker script places the primary core's initialized data in `.percore` and reserves an equally
sized area for every secondary core in `.percore_secondary`.

```
.percore : ALIGN(CACHE_LINE_SIZE) {
    __PERCORE_START__ = .;
    *(.percore .percore.*)
    . = ALIGN(CACHE_LINE_SIZE);
    __PERCORE_END__ = .;
} >image

.percore_secondary (NOLOAD) : ALIGN(CACHE_LINE_SIZE) {
    __PERCORE_SECONDARY_START__ = .;
    . += (__PERCORE_END__ - __PERCORE_START__) * (CORE_COUNT - 1);
    __PERCORE_SECONDARY_END__ = .;
} >image
```

### Usage

All cores will have their own instance of `VARIABLE` that is initialized to 1.

```rust
pub use percore::exception_free;

#[percore::percore]
static VARIABLE: ExceptionLock<RefCell<u64>> = ExceptionLock::new(RefCell::new(1));

exception_free(|token| {
    assert_eq!(1, *VARIABLE.get().borrow(token));

    *VARIABLE.get().borrow_mut(token) = 2;
    assert_eq!(2, *VARIABLE.get().borrow(token));
});
```

### Accessing from assembly

Assembly code can access the current core's instance by adding the offset in `TPIDR_EL1` to the
variable's base address. This example increments the local instance of `VARIABLE`.

```rust
global_asm!(
    "increment:
        /* Retrieve base address of the per-core variable */
        adrp	x0, {VARIABLE}
        add	x0, x0, :lo12:{VARIABLE}

        /* Add per-core offset stored in TPIDR_EL1 */
        mrs	x1, tpidr_el1
        add	x0, x0, x1
        /* At this point X0 contains the address to the local core's instance of VARIABLE */

        /* Increment the value by one. */
        ldr	x1, [x0]
        add	x1, x1, #1
        str	x1, [x0]
        ret",
    sym VARIABLE = PERCORE_BASE_VARIABLE,
);
```

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
