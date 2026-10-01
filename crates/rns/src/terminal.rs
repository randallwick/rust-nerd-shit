use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use crate::audio::Audio;
use crate::game::{Action, Game, Kind, Mode, TICK_MS};
use crate::world::{self, CHEST, Direction, HEIGHT, Pos, Room, SWORD, WIDTH};

const MIN_COLS: u16 = 64;
const MIN_ROWS: u16 = 22;
static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);

struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        // Install cleanup before changing terminal state; also restore before a
        // panic's diagnostic is printed, rather than hiding it in the alt screen.
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = restore();
            previous_hook(info);
        }));
        terminal::enable_raw_mode()?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
        let session = Self;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            Hide,
            Clear(ClearType::All)
        )?;
        Ok(session)
    }

    fn finish(self) -> io::Result<()> {
        restore()
    }
}

fn restore() -> io::Result<()> {
    if !TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
        return Ok(());
    }
    // Attempt both cleanups even when one fails (e.g. the output closed).
    let output = execute!(io::stdout(), ResetColor, Show, LeaveAlternateScreen);
    let raw = terminal::disable_raw_mode();
    output.and(raw)
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = restore();
    }
}

enum Input {
    Quit,
    Mute,
    Act(Action),
    None,
}

fn input(key: KeyEvent) -> Input {
    if key.kind == KeyEventKind::Release {
        return Input::None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c' | 'C'))
    {
        return Input::Quit;
    }
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return Input::None;
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'Q') => Input::Quit,
        KeyCode::Up | KeyCode::Char('w' | 'W') => Input::Act(Action::Move(Direction::North)),
        KeyCode::Left | KeyCode::Char('a' | 'A') => Input::Act(Action::Move(Direction::West)),
        KeyCode::Down | KeyCode::Char('s' | 'S') => Input::Act(Action::Move(Direction::South)),
        KeyCode::Right | KeyCode::Char('d' | 'D') => Input::Act(Action::Move(Direction::East)),
        KeyCode::Char(' ') => Input::Act(Action::Attack),
        KeyCode::Char('m' | 'M') if key.kind == KeyEventKind::Press => Input::Mute,
        KeyCode::Char('p' | 'P') if key.kind == KeyEventKind::Press => Input::Act(Action::Pause),
        KeyCode::Char('r' | 'R') if key.kind == KeyEventKind::Press => Input::Act(Action::Restart),
        _ => Input::None,
    }
}

pub fn run(muted: bool) -> io::Result<()> {
    let mut audio = Audio::new(muted);
    let session = TerminalSession::enter()?;
    let result = play(&mut io::stdout(), &mut audio);
    let warning = audio.warning().map(str::to_owned);
    drop(audio);
    let cleanup = session.finish();
    if let Some(warning) = warning {
        let _ = writeln!(
            io::stderr(),
            "rns: audio unavailable: {warning}; check your default output device or launch with --mute"
        );
    }
    result.and(cleanup)
}

fn play(out: &mut impl Write, audio: &mut Audio) -> io::Result<()> {
    let mut game = Game::new();
    let mut size = terminal::size()?;
    let mut last = Instant::now();
    let mut accumulator = Duration::ZERO;
    let tick = Duration::from_millis(u64::from(TICK_MS));
    audio.update(std::iter::empty(), game.mode, fits(size));
    render(out, &game, size, audio.status())?;
    loop {
        let ready = event::poll(tick.saturating_sub(accumulator))?;
        let now = Instant::now();
        // Discard excess elapsed time, including after suspend/resume. Never
        // advance more than one tick in an iteration or burst queued damage.
        let elapsed = now.duration_since(last).min(tick);
        last = now;
        if fits(size) && game.mode == Mode::Playing {
            accumulator += elapsed;
            if accumulator >= tick {
                game.tick();
                accumulator -= tick;
            }
        } else {
            accumulator = Duration::ZERO;
        }
        if ready {
            // Bound the batch so a flood of input cannot starve rendering.
            for _ in 0..32 {
                match event::read()? {
                    Event::Key(key) => match input(key) {
                        Input::Quit => return Ok(()),
                        Input::Mute => audio.toggle(),
                        Input::Act(action) if fits(size) => {
                            let old_mode = game.mode;
                            game.act(action);
                            if game.mode != old_mode {
                                accumulator = Duration::ZERO;
                            }
                        }
                        _ => {}
                    },
                    Event::Resize(cols, rows) => {
                        size = (cols, rows);
                        accumulator = Duration::ZERO;
                        last = Instant::now();
                        queue!(out, Clear(ClearType::All))?;
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        let mode = game.mode;
        audio.update(game.drain_sounds(), mode, fits(size));
        render(out, &game, size, audio.status())?;
    }
}

fn fits((cols, rows): (u16, u16)) -> bool {
    cols >= MIN_COLS && rows >= MIN_ROWS
}

fn line(out: &mut impl Write, row: u16, text: &str, width: u16) -> io::Result<()> {
    // All game text is ASCII, so bytes equal terminal columns. Leave the final
    // column unused to avoid autowrap on narrow terminals.
    let count = text.len().min(usize::from(width.saturating_sub(1)));
    queue!(
        out,
        MoveTo(0, row),
        ResetColor,
        Clear(ClearType::CurrentLine),
        Print(&text[..count])
    )
}

fn render(out: &mut impl Write, game: &Game, size: (u16, u16), audio: &str) -> io::Result<()> {
    if !fits(size) {
        if size.1 > 0 {
            line(
                out,
                0,
                "Paused: resize to at least 64 x 22. Q quits.",
                size.0,
            )?;
        }
        return out.flush();
    }
    line(
        out,
        0,
        &format!("THE BORROWED SWORD | {} | Sound: {audio}", game.room.name()),
        size.0,
    )?;
    let hearts = (0..3)
        .map(|i| if i < game.hearts { "<3 " } else { "-- " })
        .collect::<String>();
    line(
        out,
        1,
        &format!(
            "Hearts: {hearts}  Sword: {}  Facing: {:?}",
            if game.sword { "equipped" } else { "none" },
            game.facing
        ),
        size.0,
    )?;
    for y in 0..HEIGHT {
        queue!(out, MoveTo(0, (y + 2) as u16))?;
        for x in 0..WIDTH {
            let pos = Pos::new(x, y);
            let mut glyph = world::tile(game.room, pos);
            let mut color = match glyph {
                '#' => Color::DarkGrey,
                'T' => Color::Green,
                '~' => Color::Blue,
                'H' => Color::Yellow,
                _ => Color::DarkGrey,
            };
            if game.room == Room::Cave && !game.sword && pos == SWORD {
                glyph = '/';
                color = Color::White;
            }
            if game.room == Room::Secret && pos == CHEST {
                glyph = '$';
                color = if game.actors().iter().any(|e| e.alive) {
                    Color::DarkYellow
                } else {
                    Color::Yellow
                };
            }
            if let Some(enemy) = game.actors().iter().find(|e| e.alive && e.pos == pos) {
                glyph = match enemy.kind {
                    Kind::Slime => 's',
                    Kind::Beetle => 'b',
                };
                color = Color::Red;
            }
            if game.slash_ms > 0 && pos == game.player.step(game.facing) {
                glyph = '/';
                color = Color::White;
            }
            if pos == game.player {
                glyph = '@';
                color = if game.immune_ms > 0 && (game.immune_ms / 100) % 2 == 0 {
                    Color::DarkCyan
                } else {
                    Color::Cyan
                };
            }
            queue!(out, SetForegroundColor(color), Print(glyph), Print(' '))?;
        }
        queue!(out, ResetColor, Clear(ClearType::UntilNewLine))?;
    }
    line(
        out,
        18,
        "WASD/arrows: move  Space: sword  P: pause  M: mute  Q: quit",
        size.0,
    )?;
    let message = game.message();
    line(out, 19, message[0], size.0)?;
    line(out, 20, message[1], size.0)?;
    line(
        out,
        21,
        "@ you  s slime  b beetle  / sword  H hermit  $ certificate",
        size.0,
    )?;
    let overlay = match game.mode {
        Mode::Playing => None,
        Mode::Paused => Some([
            "PAUSED",
            "Even heroes need a compile break.",
            "P: resume     Q: quit",
        ]),
        Mode::Dead => Some([
            "YOU HAVE BEEN DEFEATED",
            "By an extremely unqualified opponent.",
            "R: fresh adventure     Q: quit",
        ]),
        Mode::Won => Some([
            "CERTIFICATE OF UNREASONABLE CONFIDENCE",
            "Awarded for outstanding sword-based overengineering.",
            "R: play again     Q: quit",
        ]),
    };
    if let Some(lines) = overlay {
        for (index, text) in ["", lines[0], lines[1], "", lines[2], ""]
            .iter()
            .enumerate()
        {
            queue!(
                out,
                MoveTo(2, 7 + index as u16),
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::Yellow),
                Print(format!("{text:^56}")),
                ResetColor
            )?;
        }
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_ignore_releases_and_accept_ctrl_c() {
        let mut key = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
        assert!(matches!(
            input(key),
            Input::Act(Action::Move(Direction::North))
        ));
        key.kind = KeyEventKind::Release;
        assert!(matches!(input(key), Input::None));
        assert!(matches!(
            input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Input::Quit
        ));
    }

    #[test]
    fn rendering_handles_small_terminal_and_game_overlays() {
        let mut game = Game::new();
        let mut output = Vec::new();
        render(&mut output, &game, (40, 10), "muted").unwrap();
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Paused: resize")
        );
        for mode in [Mode::Playing, Mode::Paused, Mode::Dead, Mode::Won] {
            game.mode = mode;
            let mut output = Vec::new();
            render(&mut output, &game, (64, 22), "muted").unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.contains("THE BORROWED SWORD"));
            if mode == Mode::Won {
                assert!(output.contains("CERTIFICATE OF UNREASONABLE CONFIDENCE"));
            }
        }
    }
}
