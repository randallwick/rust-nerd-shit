mod audio;
mod game;
mod terminal;
mod world;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

const HELP: &str = "rns - The Borrowed Sword

Usage: rns [--mute | --help | --version]

Launch a tiny real-time terminal adventure. Find a sword, explore the
looping woods, and claim the Certificate of Unreasonable Confidence.

Controls:
  Arrow keys / WASD   Move and face
  Space              Strike with the sword
  P                  Pause / resume
  M                  Mute / unmute (retry unavailable audio)
  R                  Restart after death or victory
  Q / Escape / Ctrl-C Quit

Requires interactive stdin and stdout, with at least 64 columns and
22 rows. Uses an alternate terminal screen and raw keyboard input;
restores the terminal on exit. No files are saved or network calls made.
Music and sound effects play through your default audio output. Use
--mute to start silently without opening an audio device. If audio is
unavailable, the game remains playable; M retries the device.
Enemy movement follows deterministic patterns; this is a game, and its
novelty output is intentional. Help and version also work through pipes.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            let _ = writeln!(io::stderr(), "rns: {message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<(), (u8, String)> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let text = match args.as_slice() {
        [] => None,
        [arg] if arg == "--mute" => None,
        [arg] if arg == "--help" => Some(HELP.to_owned()),
        [arg] if arg == "--version" => Some(format!("rns {}\n", env!("CARGO_PKG_VERSION"))),
        _ => return Err((2, "unexpected arguments; run `rns --help` for usage".into())),
    };
    if let Some(text) = text {
        return io::stdout()
            .write_all(text.as_bytes())
            .map_err(|e| (1, format!("cannot write output: {e}")));
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err((1, "gameplay requires interactive stdin and stdout; run `rns` directly in a terminal, or use `rns --help`".into()));
    }
    let muted = args.first().is_some_and(|arg| arg == "--mute");
    terminal::run(muted).map_err(|e| {
        (
            1,
            format!("terminal session failed: {e}; check your terminal and try again"),
        )
    })
}
