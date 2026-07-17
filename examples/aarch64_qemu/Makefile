# Copyright 2026 The percore Authors.
# This project is dual-licensed under Apache 2.0 and MIT terms.
# See LICENSE-APACHE and LICENSE-MIT for details.

.PHONY: build

all: target/static_array.bin

build:
	cargo build

target/static_array.bin: build
	cargo objcopy --bin static_array -- -O binary $@

qemu_static_array: target/static_array.bin
	qemu-system-aarch64 \
		-machine virt \
		-cpu max \
		-smp 2 \
		-serial mon:stdio \
		-display none \
		-kernel $<
