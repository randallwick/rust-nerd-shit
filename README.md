# rust-nerd-shit

Tiny Rust CLI tools. Disproportionate confidence.

At some point, being told to “rewrite it in Rust” stops being advice and becomes
a declaration of war. This repository is the paperwork.

The plan: build small, useful command-line tools for tasks that barely warranted
a shell alias, then enjoy the moral authority of having compiled them.

## Objectively superior

An objectively superior implementation of
[SwingingVideoPlayer](https://github.com/timbenniks/SwingingVideoPlayer), an HTML5
video player with Flash fallback.

Our proposed architecture eliminates video playback entirely, thereby removing
buffering, codec compatibility, and the temptation to install Flash. The two
projects solve unrelated problems, which has done nothing to shake our confidence.

This is friendly engineering banter. Actual performance claims will need actual
measurements. The smugness is available immediately.

## What belongs here

- Small Rust binaries that do one thing well.
- Useful output you can pipe into the next command.
- Clear help, sensible defaults, and errors that tell you what to fix.
- Dependencies that earn their place. A framework is a lot of commitment for a bit.
- Jokes in the documentation; predictable behavior in the terminal.

## Current status

Documentation only. No tools, Cargo workspace, or installation commands yet.
The first tool will establish the workspace; each tool will live in its own crate
under `crates/` and get a working usage example here.

Zero runtime bugs so far. A suspiciously easy benchmark to win.

## Development

Use stable Rust and Cargo. Once the first crate lands, the standard checks will be:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

These commands require the future Cargo workspace; they do not run against this
documentation-only skeleton.

See [AGENTS.md](AGENTS.md) for implementation guidance.
