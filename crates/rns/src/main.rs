mod audio;
mod game;
mod graphical;
mod sprites;
mod terminal;
mod world;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

const HELP: &str = "rns - The Borrowed Sword

Usage: rns [--terminal] [--mute]
       rns --help | --version

Launch a tiny sprite-based adventure in a native window. Find a sword, explore the
looping woods, and claim the Certificate of Unreasonable Confidence.
From the clearing, explore north for a stronger sword or south for armor.

Controls:
  Arrow keys / WASD   Move and face
  Space              Strike with the sword
  E                  Greet nearby peaceful wildlife
  P                  Pause / resume
  M                  Mute / unmute (retry unavailable audio)
  R                  Restart after death or victory
  Q / Escape / Ctrl-C Quit

The window uses crisp pixel scaling; resize to at least 512 x 400 to play.
Use --terminal for the ASCII version: it requires interactive stdin and
stdout, with at least 64 columns and 22 rows. It restores the terminal
on exit. No files are saved or network calls made.
Music and sound effects play through your default audio output. Use
--mute to start silently without opening an audio device. If audio is
unavailable, the game remains playable; M retries the device.
Enemy and wildlife movement follow deterministic patterns; this is a game, and its
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
        [arg] if arg == "--help" => Some(HELP.to_owned()),
        [arg] if arg == "--version" => Some(format!("rns {}\n", env!("CARGO_PKG_VERSION"))),
        _ => None,
    };
    if let Some(text) = text {
        return io::stdout()
            .write_all(text.as_bytes())
            .map_err(|e| (1, format!("cannot write output: {e}")));
    }
    let mut muted = false;
    let mut terminal = false;
    for arg in &args {
        if arg == "--mute" && !muted {
            muted = true;
        } else if arg == "--terminal" && !terminal {
            terminal = true;
        } else {
            return Err((2, "unexpected arguments; run `rns --help` for usage".into()));
        }
    }
    if !terminal {
        #[cfg(target_os = "linux")]
        if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return Err((1, "no graphical display available; launch in a desktop session, or use `rns --terminal` in an interactive terminal".into()));
        }
        macroquad::Window::from_config(graphical::config(), graphical::run(muted));
        return Ok(());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err((1, "gameplay requires interactive stdin and stdout; run `rns --terminal` directly in a terminal, or use `rns --help`".into()));
    }
    terminal::run(muted).map_err(|e| {
        (
            1,
            format!("terminal session failed: {e}; check your terminal and try again"),
        )
    })
}
