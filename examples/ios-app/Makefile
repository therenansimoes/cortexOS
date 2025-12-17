BUILD_DIR := $(PWD)/build
RUST_TARGET := aarch64-apple-ios
PROJECT_NAME := CortexOS

.PHONY: setup build deploy clean

setup:
	rustup target add $(RUST_TARGET)
	rustup target add aarch64-apple-ios-sim

build:
	mkdir -p $(BUILD_DIR)
	cargo build -p cortex-ios-ffi --target $(RUST_TARGET) --release
	@echo "✅ Binary built at: target/$(RUST_TARGET)/release/libcortex_ios_ffi.a"

build-sim:
	cargo build -p cortex-ios-ffi --target aarch64-apple-ios-sim --release

# Generate Swift bindings
bindings:
	cargo run --manifest-path ../../crates/ios-ffi/Cargo.toml \
		--example generate_bindings -- \
		generate --language swift \
		../../crates/ios-ffi/cortex.udl \
		--out-dir Generated/

deploy: build
	@echo "📱 Plug iPhone via USB-C and open Xcode"
	open $(PROJECT_NAME).xcodeproj

clean:
	rm -rf $(BUILD_DIR)
	cargo clean
