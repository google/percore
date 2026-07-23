# Copyright 2026 The percore Authors.
# This project is dual-licensed under Apache 2.0 and MIT terms.
# See LICENSE-APACHE and LICENSE-MIT for details.

.PHONY: build qemu_boxed_slice qemu_macro qemu_static_array

all: ../../target/boxed_slice.bin ../../target/static_array.bin

build:
	cargo build --target aarch64-unknown-none

../../target/boxed_slice.bin: build
	cargo objcopy --target aarch64-unknown-none --bin boxed_slice -- -O binary $@

../../target/macro.bin: build
	cargo objcopy --target aarch64-unknown-none --bin macro -- -O binary $@

../../target/static_array.bin: build
	cargo objcopy --target aarch64-unknown-none --bin static_array -- -O binary $@

qemu_boxed_slice: ../../target/boxed_slice.bin
	qemu-system-aarch64 \
		-machine virt \
		-cpu max \
		-smp 2 \
		-serial mon:stdio \
		-display none \
		-kernel $<

qemu_macro: ../../target/macro.bin
	qemu-system-aarch64 \
		-machine virt \
		-cpu max \
		-smp 2 \
		-serial mon:stdio \
		-display none \
		-kernel $<

qemu_static_array: ../../target/static_array.bin
	qemu-system-aarch64 \
		-machine virt \
		-cpu max \
		-smp 2 \
		-serial mon:stdio \
		-display none \
		-kernel $<
