KERNEL   := rustos
TARGET   := x86-rustos
QEMU     ?= qemu-system-x86_64
QEMU_ARGS ?=

DEBUG_IMG   := target/$(TARGET)/debug/bootimage-$(KERNEL).bin
RELEASE_IMG := target/$(TARGET)/release/bootimage-$(KERNEL).bin

.PHONY: all build release run run-release clean test

all: build

build:
	cargo bootimage

release:
	cargo bootimage --release

run: build
	$(QEMU) -drive format=raw,file=$(DEBUG_IMG) $(QEMU_ARGS)

run-release: release
	$(QEMU) -drive format=raw,file=$(RELEASE_IMG) $(QEMU_ARGS)

clean:
	cargo clean

test:
	cargo test
