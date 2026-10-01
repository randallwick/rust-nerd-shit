# Agent guidance

## Purpose and tone

Build tiny, useful Rust CLI tools with an unreasonable amount of confidence.
The project is a friendly response to Rust evangelism. Keep the humor playful,
aim it at language tribalism and our own overengineering, and keep people's names
out of the README. The existing unrelated-repository comparison is intentional.
Do not turn it into personal criticism or invent benchmark results.

## Current state and layout

This repository begins with documentation only. Do not claim tools, tests,
installation instructions, or release automation exist before they do.

When implementing the first requested tool:

- Create a root Cargo workspace with one binary crate per tool in `crates/<tool>/`.
- Use stable Rust and the current stable edition; avoid nightly-only features.
- Commit the workspace `Cargo.lock`, since these are applications.
- Ignore `/target/` and other generated output.
- Add the tool's purpose and copyable build, run, and usage examples to README.md.

Add shared library code only when multiple tools have a concrete need for it.
Do not scaffold speculative tools, plugin systems, services, or release pipelines.

## Implementation

- Keep each tool focused on one job. Prefer readable, idiomatic Rust over cleverness.
- Start with the standard library; use small, maintained dependencies when they
  materially simplify the implementation. Avoid async runtimes without a need.
- Prefer safe Rust. Any `unsafe` must have a concrete justification and documented
  safety invariants.
- Handle expected input, filesystem, and network failures without panicking.
  Give errors enough context to identify the problem and a useful next step.
- Keep successful output on stdout and diagnostics on stderr. Use meaningful exit
  codes, provide `--help` and `--version`, and keep piped output free of decoration.
- Keep jokes out of machine-readable output and error handling. Document any
  nondeterministic behavior or intentional novelty output.
- Avoid surprising writes, network calls, or destructive defaults. Make side
  effects explicit in the command interface and help text.

## Validation

For Rust changes, run these checks from the workspace root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Test behavior that matters: representative input, boundary cases, failure paths,
and CLI stdout, stderr, and exit codes where relevant. Smoke-test the documented
usage of a changed tool. Do not add tests merely to restate the implementation.

For documentation-only changes, check accuracy, links, and examples. Do not create
a Cargo workspace just to run checks on documentation. Report what was verified
and what could not be run; never describe an unrun check as passing.

## Working scope

Keep changes tied to the requested task and preserve unrelated user work. Update
documentation alongside behavior changes. Publishing releases, pushing commits,
or changing repository settings requires authorization from the user; an explicit
request to perform that action is sufficient.
