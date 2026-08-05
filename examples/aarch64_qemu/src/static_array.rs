// Copyright 2026 The percore Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

//! An example of using a `PerCore` for a static variable on bare-metal aarch64 to provide safe
//! mutable state for each core.

#![no_std]
#![no_main]

mod common;

use crate::common::{SECONDARY_STACK, UART};
use aarch64_rt::{entry, start_core};
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
use spin::mutex::SpinMutexGuard;

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
        let index = aff0.into();
        assert!(index < CORE_COUNT);
        index
    }
}

/// Mutable state for each core.
static STATE: PerCore<[ExceptionLock<RefCell<u32>>; CORE_COUNT], CoresImpl> =
    PerCore::new([const { ExceptionLock::new(RefCell::new(42)) }; CORE_COUNT]);

entry!(main);
/// Entry point for primary core.
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

    // Access the state for the primary core.
    exception_free(|token| {
        let mut state = STATE.get().borrow_mut(token);
        writeln!(UART.lock(), "Core 0: STATE is {}", state).unwrap();
        assert_eq!(*state, 42);
        *state += 2;
        writeln!(UART.lock(), "Core 0: Added 2, STATE is now {}", state).unwrap();
        assert_eq!(*state, 44);
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
        let state = STATE.get().borrow_mut(token);
        writeln!(UART.lock(), "Core 0: STATE is {}", state).unwrap();
        assert_eq!(*state, 44);
    });

    system_off::<Hvc>().unwrap();
    panic!("system_off returned");
}

/// Entry point for secondary core.
fn secondary_main() {
    // Access the state for the secondary core.
    exception_free(|token| {
        let mut state = STATE.get().borrow_mut(token);
        writeln!(UART.lock(), "Core 1: STATE is {}", state).unwrap();
        assert_eq!(*state, 42);
        *state -= 2;
        writeln!(UART.lock(), "Core 1: Subtracted 2, STATE is now {}", state).unwrap();
        assert_eq!(*state, 40);
    });

    SECONDARY_FINISHED.store(true, Ordering::SeqCst);

    cpu_off::<Hvc>().unwrap();
    panic!("cpu_off returned");
}
