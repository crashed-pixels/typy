# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0], and this project adheres to
[Semantic Versioning 2.0.0].

## [Unreleased]

### Added

- Compile-time explicit imports, aliases, qualified names and `lib.tp` library entries.
- Canonical module identities, shared diamond dependencies and cycle diagnostics.
- Host-supplied module loading and output APIs that preserve `no_std + alloc`.
- Native `print()` objects with variadic positional arguments and `None` results.
- Markdown language/internals guides with executable documentation examples.
- `no_std + alloc` core, embedding API, trace callbacks, and ARM/RISC-V/Wasm build checks.
- Typed functions, recursion, early returns, `None`, and `pass`.
- User classes, declared fields, constructors, bound methods, and nominal type checking.
- Checked signatures, argument counts, field assignments, and return-path coverage.
- Heap-backed call frames with a configurable recursion limit and transactional field writes.

### Changed

- File execution now requires `main.tp` declaring `def main() -> None`.
- Keep executable statements inside functions; prohibit calls in module initializers.
- Print program output only through `print()`; send file-mode debug traces to stderr.
- Retain interactive result echo and persistent evaluation in the separate REPL.
- Infer fixed binding types from initializers; preserve explicit block shadowing.
- Represent values with owned TyPy objects and shared builtin/user type descriptors.
- Split syntax, type checking, compilation, objects, VM calls, and CLI into focused modules.
- Extract a stream-testable REPL; document language limits and Rust API migration.
- Run host CI on Linux, Windows, and macOS, including release and core-only tests.

### Fixed

- Enclosing-local resolution, leaked scopes/frames, failed initializers, and stale operands (#5).
- Recoverable integer overflow and tokenizer diagnostics; static boolean-ordering checks (#5).
- Statement boundaries, blank-line/CRLF handling, and REPL buffering at EOF (#5).
- Roll back aliased instance mutations and unwind calls after runtime errors.

## [0.2.0] - 2026-08-30

### Added

- Support for `elif` branches in conditional statements, including parsing,
  static type checking, bytecode compilation, and integration coverage.
- Definition of variable with types is required now `a: int` with defaults or assign in place `a: int = 10`.
- Add scope support for block. Outer variable now available in inner scopes and can be redefined.
- CLI support for executing `.tp` source files directly via `typy main.tp`.
- File name validation enforcing `.tp` extension and snake_case naming convention.
- `tokenize_str` as an efficient borrow-based alternative to `tokenize`.
- Non-panicking `Interner::get` method as a safe alternative to `resolve`.
- Comprehensive module-level documentation and unit tests across all core components.
- Structured error handling with proper exit codes in the CLI.

### Changed

- Refactored the type checker, compiler, and VM to extract reusable helper methods, reducing code duplication.
- Replaced `unwrap` with `expect` in critical paths for clearer panic messages.
- Improved error handling in `main.rs` with structured diagnostics instead of panics.
- Extracted complex `if` statement compilation logic into a dedicated `compile_if` helper.
- Improved `README.md` with a new Features section, file execution examples, and project structure overview.

### Fixed

- Corrected parameter ordering in binary operation type error messages that previously produced malformed output.
- Improved stack underflow error messages in the VM with operand-specific context.

## [0.1.1] - 2026-08-21

### Added

- `if`/`else` conditional statements, including nested blocks.
- Multi-line input support in the REPL for indented blocks.
- `justfile` commands for common development tasks.
- Contributor documentation with formatting, linting, testing, and build
  checks.

### Changed

- Refactored statement and expression handling across the tokenizer, parser,
  type checker, compiler, and virtual machine.

### Fixed

- REPL handling of block input.

## [0.1.0] - 2026-08-15

### Added

- Initial public release of the TyPy interpreter.
- Tokenization, parsing, static type checking, bytecode compilation, and a
  stack-based virtual machine.
- Integer and boolean values, variables, arithmetic, and comparison
  expressions.
- Command-line configuration, REPL, automated tests, and continuous
  integration.
