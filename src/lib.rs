// Copyright 2024 The percpu Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

#![no_std]

mod criticalcell;
mod exceptions;

pub use self::{
    criticalcell::CriticalCell,
    exceptions::{exception_free, ExceptionFree},
};
