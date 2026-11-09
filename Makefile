# Команды rustos. `make` или `make help` — список.

KERNEL   := rustos
TARGET   := x86-rustos
QEMU     ?= qemu-system-x86_64
QEMU_ARGS ?=
SERIAL_ARGS := -serial mon:stdio

DEBUG_IMG   := target/$(TARGET)/debug/bootimage-$(KERNEL).bin
RELEASE_IMG := target/$(TARGET)/release/bootimage-$(KERNEL).bin

.DEFAULT_GOAL := help
.PHONY: help build release run run-release test fmt lint check clean

help: ## показать этот список
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "} {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

# --- Каждый день -------------------------------------------------------------

build: ## собрать образ (debug)
	cargo bootimage

release: ## собрать образ (release)
	cargo bootimage --release

run: build ## собрать и запустить в QEMU (выход: Ctrl+A X)
	$(QEMU) -drive format=raw,file=$(DEBUG_IMG) $(SERIAL_ARGS) $(QEMU_ARGS)

run-release: release ## то же с release-образом
	$(QEMU) -drive format=raw,file=$(RELEASE_IMG) $(SERIAL_ARGS) $(QEMU_ARGS)

test: ## все тесты, каждый образ запускается в QEMU
	cargo test

fmt: ## отформатировать код
	cargo fmt

lint: ## clippy для ядра и тестов, предупреждения - ошибки
	cargo clippy --all-targets -- -D warnings

# --- Перед коммитом ----------------------------------------------------------

check: ## формат, clippy и тесты разом
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

clean: ## удалить всё собранное
	cargo clean
