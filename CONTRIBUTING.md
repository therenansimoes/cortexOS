# Contributing to CortexOS

Thank you for your interest in contributing to CortexOS! This document provides guidelines and information about our development workflow.

## Development Setup

### Prerequisites

- Rust (stable toolchain)
- For WASM builds: `wasm32-wasip1` target
- For coverage: `cargo-tarpaulin`

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/therenansimoes/cortexOS.git
   cd cortexOS
   ```

2. Install required toolchain:
   ```bash
   rustup target add wasm32-wasip1
   cargo install cargo-tarpaulin  # Optional, for coverage
   ```

3. Build the project:
   ```bash
   cargo build
   ```

## CI/CD Pipeline

CortexOS uses GitHub Actions for continuous integration and deployment. All pull requests must pass the following checks:

### Automated Checks

#### 1. **Build** (`cargo build`)
- Builds all workspace crates
- Runs on Ubuntu, Windows, and macOS
- Ensures cross-platform compatibility

```bash
cargo build --workspace --all-features
```

#### 2. **Check** (`cargo check`)
- Fast compilation check without code generation
- Validates code without producing binaries

```bash
cargo check --workspace --all-features
```

#### 3. **Test** (`cargo test`)
- Runs all unit and integration tests
- Required for PR merge

```bash
cargo test --workspace --all-features
```

#### 4. **Format** (`cargo fmt`)
- Ensures consistent code formatting
- Uses default rustfmt configuration

```bash
# Check formatting
cargo fmt --all -- --check

# Auto-format code
cargo fmt --all
```

#### 5. **Lint** (`cargo clippy`)
- Static analysis and linting
- Treats warnings as errors in CI

```bash
cargo clippy --workspace --all-features -- -D warnings
```

#### 6. **WASM Build**
- Verifies that `cortex-core` compiles to WebAssembly
- Uses `wasm32-wasip1` target for WASI compatibility

```bash
cargo build --target wasm32-wasip1 -p cortex-core --no-default-features
```

#### 7. **Code Coverage**
- Generates coverage reports using `cargo-tarpaulin`
- Uploads results to Codecov
- Target: >80% coverage

```bash
cargo tarpaulin --workspace --timeout 300 --out xml --engine llvm
```

### Pre-commit Workflow

Before submitting a PR, run these commands locally:

```bash
# 1. Format your code
cargo fmt --all

# 2. Check for compilation errors
cargo check --workspace --all-features

# 3. Run tests
cargo test --workspace --all-features

# 4. Run clippy
cargo clippy --workspace --all-features -- -D warnings

# 5. Verify WASM build (optional but recommended)
cargo build --target wasm32-wasip1 -p cortex-core --no-default-features
```

### Release Automation

Releases are automated through GitHub Actions:

1. **Create a tag**: `git tag v0.1.0 && git push origin v0.1.0`
2. **Automated process**:
   - Creates a GitHub release
   - Builds binaries for multiple platforms:
     - Linux (x86_64, ARM64)
     - macOS (x86_64, ARM64)
     - Windows (x86_64)
   - Publishes crates to crates.io (if configured)

## Architecture Principles

When contributing, please follow these core principles from the [README](./README.md):

### Portability-First
- **Run anywhere**: Code must compile to native and WASM/WASI
- **Core is OS-agnostic**: Use traits and event schemas; keep platform specifics in adapters
- **Event-log first**: Model everything as timestamped events

### Design Guidelines
- **Backpressure everywhere**: Define load behavior (drop/coalesce/sample/persist)
- **Capability-based permissions**: No ambient authority
- **Open protocols**: Binary, versioned, content-addressed where possible
- **Minimal dependencies**: Prefer stability over bleeding-edge features

## Code Style

- Follow standard Rust conventions
- Use `rustfmt` for formatting (enforced in CI)
- Address all `clippy` warnings
- Write comprehensive tests for new features
- Document public APIs with doc comments

## Testing Guidelines

- Write unit tests for individual components
- Add integration tests for cross-crate functionality
- Ensure tests are deterministic and don't depend on external services
- Use descriptive test names that explain what's being tested

## Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes following the guidelines above
4. Run all pre-commit checks
5. Commit with descriptive messages
6. Push to your fork
7. Open a pull request with a clear description

### PR Requirements

- [ ] All CI checks pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings
- [ ] Tests added/updated for new functionality
- [ ] Documentation updated if needed
- [ ] WASM compatibility maintained for `cortex-core`

## Questions?

- Check the [README](./README.md) for architecture overview
- See [ROADMAP](./ROADMAP.md) for development plan
- Open an issue for questions or discussions

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).
