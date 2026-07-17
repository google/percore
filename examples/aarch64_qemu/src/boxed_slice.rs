// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

//! An example of using a `PerCore` for a static variable on bare-metal aarch64 to provide safe
//! mutable state for each core. Rather than statically allocating an array of a fixed size, this
//! example uses a boxed slice, which allows the number of cores to be determined at runtime.

#![no_std]
#![no_main]

extern crate alloc;

mod common;

use crate::common::{SECONDARY_STACK, UART, init_heap};
use aarch64_rt::{entry, start_core};
use alloc::boxed::Box;
use arm_sysregs::read_mpidr_el1;
use core::{
    cell::RefCell,
    fmt::Write,
    hint::spin_loop,
    sync::atomic::{AtomicBool, Ordering},
};
use percore::{Cores, ExceptionLock, PerCore, exception_free};
use smccc::{
    Hvc,
    psci::{cpu_off, system_off},
};
use spin::{Once, mutex::SpinMutexGuard};

/// The number of CPU cores on the system.
const CORE_COUNT: usize = 2;

/// Used to signal that the secondary core has finished running and is about to turn itself off.
static SECONDARY_FINISHED: AtomicBool = AtomicBool::new(false);

/// Implementation of the `percore::Cores` trait.
struct CoresImpl;

// SAFETY: The `core_index` implementation checks that the affinity values are within the expected
// range for a 2-core system, and then returns a unique value for each of the 2 cores.
unsafe impl Cores for CoresImpl {
    fn core_index() -> usize {
        let mpidr_el1 = read_mpidr_el1();
        assert_eq!(mpidr_el1.aff3(), 0);
        assert_eq!(mpidr_el1.aff2(), 0);
        assert_eq!(mpidr_el1.aff1(), 0);
        let aff0 = mpidr_el1.aff0();
        assert!(aff0 < 2);
        aff0.into()
    }
}

/// Mutable state for each core.
static STATE: Once<PerCore<Box<[ExceptionLock<RefCell<u32>>]>, CoresImpl>> = Once::new();

entry!(main);
fn main(arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> ! {
    writeln!(
        UART.lock(),
        "main({:#x}, {:#x}, {:#x}, {:#x})",
        arg0,
        arg1,
        arg2,
        arg3
    )
    .unwrap();

    init_heap();

    // Initialise the state for the appropriate number of cores. This could be read from the device
    // tree.
    STATE.call_once(|| PerCore::new_with_default(CORE_COUNT));

    // Access the state for the primary core.
    exception_free(|token| {
        let mut state = STATE.get().unwrap().get().borrow_mut(token);
        writeln!(UART.lock(), "Core 0: STATE is {}", state).unwrap();
        *state += 2;
        writeln!(UART.lock(), "Core 0: Added 2, STATE is now {}", state).unwrap();
    });

    let secondary_stack = SpinMutexGuard::leak(SECONDARY_STACK.try_lock().unwrap());
    let secondary_mpidr = 1;

    // Start a secondary core.
    unsafe {
        start_core::<Hvc, _, _>(secondary_mpidr, secondary_stack, secondary_main).unwrap();
    }

    // Wait for secondary core to finish.
    while !SECONDARY_FINISHED.load(Ordering::SeqCst) {
        spin_loop();
    }

    exception_free(|token| {
        let state = STATE.get().unwrap().get().borrow_mut(token);
        writeln!(UART.lock(), "Core 0: STATE is {}", state).unwrap();
    });

    system_off::<Hvc>().unwrap();
    panic!("system_off returned");
}

fn secondary_main() {
    // Access the state for the secondary core.
    exception_free(|token| {
        let mut state = STATE.get().unwrap().get().borrow_mut(token);
        writeln!(UART.lock(), "Core 1: STATE is {}", state).unwrap();
        *state += 15;
        writeln!(UART.lock(), "Core 1: Added 15, STATE is now {}", state).unwrap();
    });

    SECONDARY_FINISHED.store(true, Ordering::SeqCst);

    cpu_off::<Hvc>().unwrap();
    panic!("cpu_off returned");
}
