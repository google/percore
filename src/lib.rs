// Copyright 2024 The percpu Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

#![no_std]

mod exceptions;
mod lock;

pub use self::{
    exceptions::{exception_free, ExceptionFree},
    lock::ExceptionLock,
};
