# rust-nerd-shit

Tiny Rust tools. Disproportionate confidence.

At some point, being told to “rewrite it in Rust” stops being advice and becomes
a declaration of war. This repository is the paperwork.

The plan was to build small, useful command-line tools for tasks that barely
warranted a shell alias. The first one turned out to be a pixel-art adventure.
We remain disproportionately confident about this allocation of engineering time.

## The Borrowed Sword

`rns` launches a tiny, real-time **2D pixel-art adventure**: seven handcrafted screens, a
sword, three hearts, and monsters whose principal qualification is availability.
Find the sword cave, explore the looping woods, and claim the **Certificate of
Unreasonable Confidence** from its two extremely budget guards. The source
version now has original animated sprites, textured terrain, rippling ponds,
flowers, shadows, and peaceful rabbits, ducks, and tortoises. The original ASCII
renderer is still available with `--terminal`.

The [v0.2.0 release](https://github.com/randallwick/rust-nerd-shit/releases/tag/v0.2.0)
includes a **macOS Apple Silicon** binary with the graphical and terminal modes,
music, effects, sprites, and font embedded. The earlier
[v0.1.0 release](https://github.com/randallwick/rust-nerd-shit/releases/tag/v0.1.0)
contains the original terminal edition.
Download the archive and `SHA256SUMS` from the release page, then run:

```sh
shasum -a 256 -c SHA256SUMS
tar -xzf rns-v0.2.0-aarch64-apple-darwin.tar.gz
./rns-v0.2.0-aarch64-apple-darwin/rns
```

The macOS binary is ad-hoc signed, without Developer ID signing or notarization.

Building from source requires stable Rust (edition 2024; Rust 1.85 or newer).
Linux builds also need `pkg-config` and
ALSA development headers (for example, `libasound2-dev` on Debian/Ubuntu). macOS
uses the built-in Core Audio backend. The first build downloads dependencies
from crates.io. The graphical version also needs a desktop session and graphics
support; on Linux, install the X11, Xi, and OpenGL runtime libraries for your
distribution. The current window backend uses X11, so a Wayland desktop needs
XWayland and `DISPLAY`. Dynamically loaded graphics libraries must be discoverable
by the process. Linux runtime behavior has not yet been verified.
From the repository root:

```sh
cargo build --release --locked
./target/release/rns
```

Or build and play in development mode:

```sh
cargo run -p rns
```

The native window draws embedded **16 x 16 sprites** onto a 512 x 400 canvas,
enlarged by whole numbers with nearest-neighbor filtering. Resizing adds margins
to keep the pixels crisp; below 512 x 400, gameplay and audio pause until it fits.
Text is drawn separately at the window's resolution using the embedded
Pixelify Sans font, with retro lettering, larger dialogue and high-contrast controls.
Hold a movement key to walk. Window controls and **Q / Escape** close the game.
The seven room layouts, equipment, and woods puzzle are shared by both renderers.

For the ASCII version, use an interactive terminal with at least
**64 columns and 22 rows**:

```sh
cargo run -p rns -- --terminal
./target/release/rns --terminal --mute
```

Help and version work without an interactive terminal:

```sh
./target/release/rns --help
./target/release/rns --version
```

| Key | Action |
| --- | --- |
| Arrow keys / WASD | Move and face north, west, south, or east |
| Space | Swing forward; the frost sword reaches two tiles |
| E | Greet the closest peaceful animal within two tiles |
| P | Pause or resume |
| M | Mute / unmute music and effects; retry if audio is unavailable |
| R | Start fresh after death or victory |
| Q / Escape / Ctrl-C | Quit |

The glade's north exit leads to the sword cave. Walk onto the sword to equip it;
the hermit has also left a useful inscription. Head east from the glade to
meet the local talent. Walls, trees, and water block movement. Trying to walk
into an obstacle still changes your facing. Cacti and frozen ponds also block
movement. Cross an opening at the map's edge
to enter another screen. In terminal mode, `@` is you, `s` is a slime, `b` is a
beetle, `/` is the sword, `H` is the hermit, and `$` is the certificate. Walls
`#`, trees `T`, and water `~` block movement.

Peaceful wildlife lives in the glade, clearing, woods, desert, and snow: rabbits hop on land,
ducks stay in ponds, and tortoises take their time. They wander on a deterministic
900 ms beat, with resting beats and slower tortoises. Only wildlife in the current
room advances; revisiting keeps its state, and restarting resets it. Animals
cause no damage, cannot be killed, and never block your movement or the puzzle.
Press **E** nearby for a greeting; no items or rewards are required. In terminal
mode they appear as yellow `r`, `d`, and `t`.

Original enemies move in deterministic patterns every 600 ms, take one sword hit, and
deal one heart of contact damage. You get one second of protection after damage
or entering a screen. Your sword slash lasts 150 ms, holds your position and
facing, and can be used every 300 ms. Movement is limited to one step per 120 ms,
rounded up to the next 50 ms simulation tick. Holding a movement key uses your
terminal's key-repeat behavior in ASCII mode; the window handles held keys
directly. No special terminal keyboard protocol is required.
Defeated enemies stay defeated until you restart.

One enemy each in the clearing, desert, and snow drops a **heart container**
when defeated. Walk onto its gold-bordered heart to collect it: your maximum
health grows by one heart and your health fully refills. You start with three
hearts and can grow to six. Uncollected containers stay in their room; restarting
resets both drops and maximum health. Terminal mode marks containers with yellow `C`.

Health also regenerates slowly: **half a heart every 20 seconds** of active play,
up to your current maximum. Taking damage restarts that timer. Pausing, endings,
and an undersized window or terminal freeze regeneration along with the simulation.

### Expeditions and equipment

The clearing is now a crossroads. West returns to the glade, east enters the
looping woods, north leads to **Frostbound Hollow**, and south leads to the
**Sunburnt Dunes**. Return from the snow through its south exit, or from the
desert through its north exit. Both branches are optional; the original woods
route still leads to the certificate.

- In the desert, walk onto the **dune armor** in the south to equip it. Contact
  damage falls from one heart to **half a heart**. Hearts show partial health;
  the existing one-second protection after contact still applies.
- In the snow, walk onto the **frost sword** in the north. It deals **double
  damage** and reaches **two tiles** in the facing direction. It can be your
  first sword; visiting the cave afterward never downgrades it.

Desert beetles and frost slimes have two health points, shown by two small marks
above them. They need **two separate swings** with the basic sword or one with
the frost sword. Repeated contact with the same blade during one swing counts
as one hit. Walls, cacti, trees, water, and ice stop blade reach. The visual
sweep follows the hero's hand; attacks still hit forward tiles, not enemies
beside or behind you. The longer frost blade has a blue trail.

Equipment stays equipped between screens and resets when you restart. HUD icons
show owned armor and the current sword. Terminal mode shows armor as `A`, the
frost pickup as `!`, and half hearts as `1/2`.

The adventure intentionally produces colorful novelty output. Terminal gameplay
requires interactive stdin and stdout; piping it reports a plain error instead. It uses
the alternate screen and raw input, restoring the terminal on exit, Ctrl-C,
errors, or panic unwinding. Shrinking the terminal below the minimum size pauses
the simulation until it fits again. There are no saves, file writes, network
calls, random maps, or random enemy or wildlife movements at runtime. The native
window uses Macroquad for graphics and the same embedded audio as terminal mode.

### Music and sound effects

The original theme, **A Most Unnecessary Quest**, is a 16-bar, 144 BPM adventure
loop in A minor: pulse-wave melody and arpeggios, triangle bass, and tiny drums.
Sword swings, enemy hits, damage, item pickups, secret-room discovery, death,
and victory each have their own synthesized effect.

Music and effects play by default through your system's default audio output.
Press **M** to mute or unmute. To launch silently without opening an audio device:

```sh
cargo run -p rns -- --mute
./target/release/rns --mute
```

Pause and undersized windows or terminals suspend playback. Death and victory pause the
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

The Cargo workspace contains one binary crate, `crates/rns/`. Release v0.2.0
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
python3 tools/generate_sprites.py --check
```

Tests cover map reachability, movement, sword combat, enemy patterns, heart drops,
health capacity and regeneration, pause/restart, the woods sequence, victory,
peaceful wildlife, greetings, sprite
decoding, pixel scaling, frame timing, rendering, and CLI output and exit
codes. Audio tests also check cue timing, decoding, volume bounds, loop seams,
mute/pause/endings, and the MIDI's tempo, duration, and balanced note events.
Interactive smoke checks should also cover a complete adventure, terminal
resizing, pause/resume, mute/unmute, and terminal restoration after quitting or
Ctrl-C. Automated audio tests use an in-memory mixer and need no audio device.

The original pixel-art source is [tools/generate_sprites.py](tools/generate_sprites.py).
Run `python3 tools/generate_sprites.py` to regenerate the embedded PNG atlas and
its Rust sprite index. It uses only Python's standard library. There are no
downloaded game sprites or runtime asset files.

The embedded UI font is [Pixelify Sans](https://github.com/eifetx/Pixelify-Sans),
distributed under the
[SIL Open Font License](crates/rns/assets/fonts/PixelifySans-OFL.txt).

To smoke-test the real GPU renderer, capture all seven rooms, greetings, animation,
equipment, heart drops and pickups, six-heart health and regeneration,
every phase of both swords in all directions, overlays, restart, and
resizing in a native window:

```sh
cargo run -p rns --example graphics_smoke -- /tmp/rns-sprite-qa
```

This development harness explicitly writes PNG frames into the supplied
directory and closes its window when complete. It requires a graphical desktop;
regular game launches never write screenshots or other files.

See [AGENTS.md](AGENTS.md) for the source layout, gameplay contracts, and setup
and validation notes for continuing on another machine.
