.PHONY: help build test release install clean fmt lint check run dev

# Default target
help:
	@echo "Available targets:"
	@echo "  build     - Build the project in debug mode"
	@echo "  release   - Build the project in release mode"
	@echo "  test      - Run all tests"
	@echo "  install   - Install the binary to ~/.cargo/bin"
	@echo "  clean     - Clean build artifacts"
	@echo "  fmt       - Format code with rustfmt"
	@echo "  lint      - Run clippy linter"
	@echo "  check     - Run format check and linter"
	@echo "  run       - Run the application with a sample PR"
	@echo "  dev       - Run in development mode with debug output"

# Build targets
build:
	cargo build

release:
	cargo build --release

# Testing
test:
	cargo test --all-features

test-verbose:
	cargo test --all-features -- --nocapture

# Installation
install: release
	cargo install --path .

install-local: release
	mkdir -p ~/.local/bin
	cp target/release/revu ~/.local/bin/
	@echo "Installed to ~/.local/bin/revu"

# Maintenance
clean:
	cargo clean
	rm -f Cargo.lock

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

check: fmt lint
	cargo check --all-features

# Running
run:
	@echo "Usage: cargo run -- <PR_URL>"
	@echo "Example: cargo run -- https://github.com/rust-lang/rust/pull/12345"

dev:
	REVU_DEBUG=1 RUST_LOG=debug cargo run -- $(ARGS)

# CI tasks
ci: check test
	@echo "All CI checks passed!"

# Documentation
doc:
	cargo doc --no-deps --open

# Benchmarking (if you add benchmarks)
bench:
	cargo bench

# Update dependencies
update:
	cargo update
	cargo audit

# Release preparation
prepare-release:
	@echo "Preparing for release..."
	@echo "1. Update version in Cargo.toml"
	@echo "2. Update CHANGELOG.md"
	@echo "3. Commit changes"
	@echo "4. Create tag: git tag -a v\$${VERSION} -m 'Release v\$${VERSION}'"
	@echo "5. Push tag: git push origin v\$${VERSION}"