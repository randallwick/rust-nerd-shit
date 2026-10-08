# Agent guidance

## Purpose and tone

Build tiny, useful Rust CLI tools with an unreasonable amount of confidence.
The project is a friendly response to Rust evangelism. Keep the humor playful,
aim it at language tribalism and our own overengineering, and keep people's names
out of the README. Do not restore comparisons with other people's repositories
or invent benchmark results.

## Current state and layout

The root Cargo workspace contains one binary, `crates/rns/`: **The Borrowed
Sword**. It defaults to a Macroquad pixel-art window; `--terminal` selects the
original ASCII renderer, and `--mute` skips opening an audio device. Use stable
Rust (edition 2024; declared minimum 1.85), keep `Cargo.lock` committed, and keep
generated build output under ignored `/target/`.

- `src/game.rs` and `src/game/tests.rs`: deterministic shared simulation and tests.
- `src/world.rs`: seven room layouts, terrain, exits, and pickup positions.
- `src/graphical.rs`: native rendering, input, integer scaling, and frame clock.
- `src/terminal.rs`: ASCII rendering, input, and terminal restoration.
- `src/audio.rs`: embedded music/effects and nonfatal audio-device handling.
- `src/main.rs` and `tests/cli.rs`: argument handling and CLI behavior.
- `assets/`: embedded audio, sprite atlas, and licensed Pixelify Sans font.
- `tools/generate_sprites.py`: original pixel art and generated `src/sprites.rs`.
- `tools/generate_audio.py`: original MIDI score and synthesized WAV assets.

Paths above are relative to `crates/rns/`, except `tools/`, which is at the
workspace root. Edit the sprite generator rather than the generated atlas/index.
No runtime asset downloads, saves, services, or release automation exist.
The v0.2.0 macOS Apple Silicon archive includes the graphical and gameplay
upgrades; v0.1.0 is the earlier terminal edition. Releases are packaged manually.

Keep one binary crate per concrete tool under `crates/<tool>/`, and update the
README with copyable build, run, and usage examples alongside behavior changes.

Add shared library code only when multiple tools have a concrete need for it.
Do not scaffold speculative tools, plugin systems, services, or release pipelines.

## Gameplay contracts

- Keep gameplay in the shared simulation, advancing in 50 ms ticks. Renderers
  must freeze simulation and audio while undersized; pause, death, and victory
  freeze gameplay timers. Do not tie combat or regeneration to rendering FPS.
- The woods puzzle counts screen-edge crossings: north, west, south, west.
  Wrong non-east exits reset the sequence; east escapes to the clearing.
- Wildlife stays deterministic, room-local, habitat-constrained, harmless,
  unkillable, and nonblocking. E greets the closest animal within two tiles.
- Desert armor halves contact damage. The snow sword deals double damage and
  reaches two forward tiles, stopped by blocking terrain. Tough biome enemies
  require two basic swings; one enemy takes damage only once per swing.
- Three designated enemies (clearing, desert, snow) drop permanent heart
  containers. Pickup adds one maximum heart, caps at six, and fully heals.
  Drops, kills, equipment, and capacity persist between rooms and reset on restart.
- Health uses whole hearts plus an optional half heart. Regenerate half a heart
  every 20 seconds of active play; damage resets the timer. Regeneration is quiet.
- Preserve hand-anchored sword sweeps, crisp 16 x 16 sprites on the 512 x 400
  canvas, and high-contrast Pixelify Sans text drawn at window resolution.
- Audio-device failures must leave the game playable. M toggles/retries audio;
  `--mute` avoids opening a device. Keep all asset licensing in the repository.

## Picking up on another machine

Fetch and check out the PR branch before starting; confirm `git status` and
preserve any existing local work. Build on that machine with
`cargo build --release --locked`, then run `./target/release/rns --mute` for an
initial graphics check. Do not copy the macOS release binary onto Linux.

The implementation branch is `feat/initial-implementation`. For a fresh checkout:

```sh
git clone --branch feat/initial-implementation git@github.com:randallwick/rust-nerd-shit.git
cd rust-nerd-shit
cargo build --release --locked
./target/release/rns --mute
```

Once the PR merges, use the updated `main` branch for new work.

The game's Linux dependencies are stable Rust
and Cargo with rustfmt/Clippy, a C toolchain, `pkg-config`, ALSA development
headers, and X11/Xi/OpenGL runtime libraries discoverable by the process.
The current Miniquad backend is X11, requiring a valid `DISPLAY`; a Wayland
desktop therefore also needs XWayland. Python 3 is required for asset generation
and its checks, but not to play the game.

Run the native game and graphics smoke harness in a local desktop session.
For SSH without a display, use `./target/release/rns --terminal --mute` in an
interactive terminal at least 64 x 22; unit tests and asset checks need no desktop.
Current graphics/runtime validation was on macOS Apple Silicon.
Recheck rendering, font readability, resize/pause behavior, and audio on Linux
before claiming Linux runtime support is verified.

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
python3 -m unittest discover -s tools -p 'test_*.py'
python3 tools/generate_audio.py --check
python3 tools/generate_sprites.py --check
```

Test behavior that matters: representative input, boundary cases, failure paths,
and CLI stdout, stderr, and exit codes where relevant. Smoke-test the documented
usage of a changed tool. Do not add tests merely to restate the implementation.

For graphics changes, run the native harness in a desktop session and inspect
its PNGs (the supplied directory is the only screenshot output):

```sh
cargo run -p rns --example graphics_smoke -- /tmp/rns-sprite-qa
```

It covers all rooms, wildlife, both sword sweeps, equipment, heart drops/pickups,
six-heart health, regeneration, overlays, restart, and resizing. On macOS a
sandboxed native launch may abort in Cocoa; use authorized desktop access for
the game's own test window. Smoke-test terminal mode in a real terminal or PTY,
including pause/resume, resize, quit, and restored terminal settings. Automated
audio tests use an in-memory mixer; they do not verify audible speaker output.

For documentation-only changes, check accuracy, links, and examples. Do not create
a Cargo workspace just to run checks on documentation. Report what was verified
and what could not be run; never describe an unrun check as passing.

## Working scope

Keep changes tied to the requested task and preserve unrelated user work. Update
documentation alongside behavior changes. Publishing releases, pushing commits,
or changing repository settings requires authorization from the user; an explicit
request to perform that action is sufficient.
