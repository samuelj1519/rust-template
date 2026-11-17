# Rust conventions

Workspace lints enforce the items in the first list. This page lists those lints and states the defaults that no lint can check.

## Enforced by lints

`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` denies every lint in this list.

- `clippy::wildcard_imports`
- `clippy::enum_glob_use`
- `clippy::ptr_arg`
- `clippy::borrowed_box`
- `clippy::ref_option`
- `clippy::redundant_clone`
- `clippy::needless_pass_by_value`
- `clippy::fn_params_excessive_bools`
- `clippy::struct_excessive_bools`
- `clippy::derivable_impls`
- `clippy::expl_impl_clone_on_copy`
- `missing_debug_implementations`
- `clippy::must_use_candidate`
- `clippy::unwrap_used`
- `clippy::expect_used`
- `clippy::unnecessary_unwrap`
- `missing_docs`
- `unsafe_code`
- `clippy::allow_attributes_without_reason`

The pinned stable toolchain rejects `#![feature]`.

## Defaults that lints do not check

Borrow a value before you take ownership of it. Take ownership when the function must keep the value. Use `Rc` or `Arc` only when more than one owner must share it. Copy a value last.

Define a trait for dynamic dispatch or a generic bound. Put related functions in a module when a trait is not required.

Use a macro when a function and a generic cannot express the code. Prefer the function or the generic otherwise.

Write a `for` or `while` loop when an iterator chain is hard to read.

## Applications and libraries

This template sets `publish = false`. Match types exhaustively. Leave `#[non_exhaustive]` off.

Add `#[non_exhaustive]` on a type only in a library you publish for other projects. Downstream `match` expressions then need a wildcard arm. A new variant still compiles in those projects. The compiler does not report the missing arm.

## Rules left out

This template leaves out these Chromium rules.

- Small crates and a flat directory.
- A ban on `async`.
- A ban on threads started from `std`.

Split a crate after a measurement shows a gain. Use `cargo build --timings` or a unit graph.

## Sources

The upstream text is the Google Rust Style Guide, <https://google.github.io/styleguide/rustguide.html>, fetched on 2026-11-17.

Items taken from that text:

- No glob imports.
- No `&String`, `&Vec<T>`, `&Box<T>`, or `&Option<T>` parameters.
- No redundant `clone`.
- No by-value parameter that the function only reads.
- An enum for a run of boolean parameters.
- `#[derive]` for standard traits.
- `#[must_use]` on a function whose return value matters.
- Prefer `expect` to `unwrap`, and pattern matching to a check followed by `unwrap`.
- Documentation on public items.
- No `unsafe`.
- No unstable features.
- A reason on every lint exception.
- Borrow, then own, then `Rc` or `Arc`, then copy.
- A trait for polymorphism or a generic bound.
- A macro only when a function and a generic cannot express the code.
- A direct loop when an iterator chain is unclear.
- Exhaustive matches in an application, and `#[non_exhaustive]` only for a published library.

Changes from that text:

- Enforced items are lint names. This page does not restate those rules.
- `missing_docs`, `clippy::redundant_clone`, and `clippy::allow_attributes_without_reason` are `warn` in `[workspace.lints]`. Their default level is allow.
- `clippy::unwrap_used` and `clippy::expect_used` stay deny outside tests. `clippy.toml` allows both inside tests.
- `#[non_exhaustive]` is off by default. An application keeps exhaustive matches.
- The Chromium rules for crate size, `async`, and `std` threads are omitted. A crate split waits for a build measurement.

A secondary article used to locate these items, and not copied here, is Tony Bai, <https://mp.weixin.qq.com/s/hMIpB__AUYmPQSKbw6cCcw>.
