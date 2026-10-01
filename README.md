# rust-nerd-shit

Tiny Rust CLI tools. Disproportionate confidence.

At some point, being told to “rewrite it in Rust” stops being advice and becomes
a declaration of war. This repository is the paperwork.

The plan was to build small, useful command-line tools for tasks that barely
warranted a shell alias. The first one turned out to be a secret terminal game.
We remain disproportionately confident about this allocation of engineering time.

## Objectively superior

An objectively superior implementation of
[SwingingVideoPlayer](https://github.com/timbenniks/SwingingVideoPlayer), an HTML5
video player with Flash fallback.

Our proposed architecture eliminates video playback entirely, thereby removing
buffering, codec compatibility, and the temptation to install Flash. The two
projects solve unrelated problems, which has done nothing to shake our confidence.

This is friendly engineering banter. Actual performance claims will need actual
measurements. The smugness is available immediately.

## The Borrowed Sword

`rns` launches a tiny, real-time ASCII adventure: five handcrafted screens, a
sword, three hearts, and monsters whose principal qualification is availability.
Find the sword cave, explore the looping woods, and claim the **Certificate of
Unreasonable Confidence** from its two extremely budget guards.

Playing requires an interactive terminal with at least **64 columns and 22 rows**.
The [v0.1.0 release](https://github.com/randallwick/rust-nerd-shit/releases/tag/v0.1.0)
includes a **macOS Apple Silicon** binary, with music and effects embedded.
Download the archive and `SHA256SUMS` from the release page, then run:

```sh
shasum -a 256 -c SHA256SUMS
tar -xzf rns-v0.1.0-aarch64-apple-darwin.tar.gz
./rns-v0.1.0-aarch64-apple-darwin/rns
```

The macOS binary is ad-hoc signed, without Developer ID signing or notarization.

Building from source requires stable Rust (edition 2024; Rust 1.85 or newer).
Linux builds also need `pkg-config` and
ALSA development headers (for example, `libasound2-dev` on Debian/Ubuntu). macOS
uses the built-in Core Audio backend. The first build downloads dependencies
from crates.io. From the repository root:

```sh
cargo build --release
./target/release/rns
```

Or build and play in development mode:

```sh
cargo run -p rns
```

Help and version work without an interactive terminal:

```sh
./target/release/rns --help
./target/release/rns --version
```

| Key | Action |
| --- | --- |
| Arrow keys / WASD | Move and face north, west, south, or east |
| Space | Strike the adjacent tile in the direction you face |
| P | Pause or resume |
| M | Mute / unmute music and effects; retry if audio is unavailable |
| R | Start fresh after death or victory |
| Q / Escape / Ctrl-C | Quit |

The glade's north exit leads to the sword cave. Walk onto `/` to equip the sword;
the hermit `H` has also left a useful inscription. Head east from the glade to
meet the local talent. `@` is you, `s` is a slime, `b` is a beetle, and `$` is the
certificate. Walls `#`, trees `T`, and water `~` block movement. Trying to walk
into an obstacle still changes your facing. Cross an opening at the map's edge
to enter another screen.

Enemies move in deterministic patterns every 600 ms, take one sword hit, and
deal one heart of contact damage. You get one second of protection after damage
or entering a screen. Your sword slash lasts 150 ms, holds your position and
facing, and can be used every 300 ms. Movement is limited to one step per 120 ms,
rounded up to the next 50 ms simulation tick. Holding a movement key uses your
terminal's key-repeat behavior; no special keyboard protocol is required.
Defeated enemies stay defeated until you restart.

The adventure intentionally produces colorful novelty output. Gameplay requires
interactive stdin and stdout; piping it reports a plain error instead. It uses
the alternate screen and raw input, restoring the terminal on exit, Ctrl-C,
errors, or panic unwinding. Shrinking the terminal below the minimum size pauses
the simulation until it fits again. There are no saves, file writes, network
calls, random maps, or random enemy movements at runtime.

### Music and sound effects

The original theme, **A Most Unnecessary Quest**, is a 16-bar, 144 BPM adventure
loop in A minor: pulse-wave melody and arpeggios, triangle bass, and tiny drums.
Sword swings, enemy hits, damage, sword pickup, secret-room discovery, death,
and victory each have their own synthesized effect.

Music and effects play by default through your system's default audio output.
Press **M** to mute or unmute. To launch silently without opening an audio device:

```sh
cargo run -p rns -- --mute
./target/release/rns --mute
```

Pause and undersized terminals suspend playback. Death and victory pause the
theme and play their own ending; restarting starts the theme over. Muting clears
active effects while the theme's clock continues with gameplay. If no audio
device can be opened, or the device disconnects, the game continues silently
with `Sound: unavailable`. **M** retries; a diagnostic appears after you quit.
Audio stops when the game exits. Use your system volume control for loudness.

The editable [MIDI score](crates/rns/assets/audio/quest.mid) and the
[rendered theme](crates/rns/assets/audio/quest.wav) are included alongside the
effects in `crates/rns/assets/audio/`. The game embeds the WAV assets generated
from the same score, so it needs no MIDI synthesizer, soundfont, external player,
or runtime asset files. Other MIDI players may use different instrument sounds.
The music and effects are original synthesized assets, not sampled game audio.

To regenerate the MIDI and WAV files with Python's standard library:

```sh
python3 tools/generate_audio.py
```

<details>
<summary>Puzzle spoiler: the secret route</summary>

From the clearing, enter the woods to the east. Cross the woods' screen edges
**north, west, south, west**, in that order. Each of the first three exits loops
back to the same woods screen; the final west exit reveals the secret room.
Walking in those directions without crossing the screen edges does not count.

A wrong north, west, or south exit resets the sequence completely; the mistaken
exit does not begin a new attempt. East always escapes to the clearing and resets
progress. Reentering the woods starts a fresh attempt. Loops do not respawn
enemies. Defeat both secret-room guards, then walk onto the certificate to win.
If you forgot your sword, leave east and return to the glade's cave.

</details>

## What belongs here

- Small Rust binaries that do one thing well.
- Useful output you can pipe into the next command, or an explicitly interactive
  game that has misplaced its business justification.
- Clear help, sensible defaults, and errors that tell you what to fix.
- Dependencies that earn their place. A framework is a lot of commitment for a bit.
- Jokes in the documentation and the adventure; predictable CLI errors.

## Current status

The Cargo workspace contains one binary crate, `crates/rns/`. Release v0.1.0
provides a macOS Apple Silicon binary; other platforms can build from source.
Releases are packaged manually. The workspace lockfile is included for
reproducible application builds.

## Development

Use stable Rust and Cargo, with rustfmt and Clippy available. Run from the
workspace root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 -m unittest discover -s tools -p 'test_*.py'
python3 tools/generate_audio.py --check
```

Tests cover map reachability, movement, sword combat, enemy patterns, health,
pause/restart, the woods sequence, victory, rendering, and CLI output and exit
codes. Audio tests also check cue timing, decoding, volume bounds, loop seams,
mute/pause/endings, and the MIDI's tempo, duration, and balanced note events.
Interactive smoke checks should also cover a complete adventure, terminal
resizing, pause/resume, mute/unmute, and terminal restoration after quitting or
Ctrl-C. Automated audio tests use an in-memory mixer and need no audio device.

See [AGENTS.md](AGENTS.md) for implementation guidance.
