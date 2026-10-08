//! Native sprite renderer. Gameplay stays in the same deterministic simulation
//! used by the terminal version; assets are embedded and never loaded from disk.
use macroquad::prelude::*;

use crate::audio::Audio;
use crate::game::{Action, CritterKind, Game, Kind, Mode, SLASH_MS, TICK_MS};
use crate::sprites::Sprite;
use crate::world::{self, ARMOR, CHEST, Direction, FROST_SWORD, HEIGHT, Pos, Room, SWORD, WIDTH};

pub(crate) const CANVAS_W: f32 = 512.0;
pub(crate) const CANVAS_H: f32 = 400.0;
const TILE: f32 = 16.0;
const MAP_X: f32 = 16.0;
const MAP_Y: f32 = 64.0;
const ATLAS: &[u8] = include_bytes!("../assets/sprites/atlas.png");
const UI_FONT: &[u8] = include_bytes!("../assets/fonts/PixelifySans.ttf");
const INK: Color = color_u8!(23, 32, 37, 255);
const PANEL: Color = color_u8!(34, 46, 48, 255);
const GOLD: Color = color_u8!(241, 205, 133, 255);
const PAPER: Color = color_u8!(225, 223, 199, 255);
const MUTED: Color = color_u8!(185, 202, 192, 255);

pub fn config() -> Conf {
    Conf {
        window_title: "The Borrowed Sword".into(),
        window_width: (CANVAS_W * 2.0) as i32,
        window_height: (CANVAS_H * 2.0) as i32,
        window_resizable: true,
        high_dpi: true,
        sample_count: 1,
        ..Default::default()
    }
}

pub async fn run(muted: bool) {
    prevent_quit();
    let mut audio = Audio::new(muted);
    let mut game = Game::new();
    let scene = match Scene::new() {
        Ok(scene) => scene,
        Err(error) => {
            eprintln!("rns: cannot initialize graphics: {error}");
            return;
        }
    };
    let mut clock = Clock::default();
    loop {
        if is_quit_requested()
            || is_key_pressed(KeyCode::Q)
            || is_key_pressed(KeyCode::Escape)
            || ((is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
                && is_key_pressed(KeyCode::C))
        {
            break;
        }
        let fits = window_fits();
        // Input precedes simulation: pausing never permits one last enemy step.
        if is_key_pressed(KeyCode::M) {
            audio.toggle();
        }
        let old_mode = game.mode;
        if fits {
            if is_key_pressed(KeyCode::P) {
                game.act(Action::Pause);
            }
            if is_key_pressed(KeyCode::R) {
                game.act(Action::Restart);
            }
            if is_key_pressed(KeyCode::E) {
                game.act(Action::Greet);
            }
            if is_key_pressed(KeyCode::Space) {
                game.act(Action::Attack);
            }
            let direction = held_direction();
            if let Some(direction) = direction {
                game.act(Action::Move(direction));
            }
        }
        if old_mode != game.mode {
            clock.reset();
        }
        clock.advance(&mut game, get_frame_time(), fits);
        let mode = game.mode;
        audio.update(game.drain_sounds(), mode, fits);
        scene.draw(&game, audio.status(), fits, clock.fraction());
        next_frame().await;
    }
    if let Some(warning) = audio.warning() {
        eprintln!(
            "rns: audio unavailable: {warning}; check your output device or launch with --mute"
        );
    }
}

fn held_direction() -> Option<Direction> {
    // Prefer a newly pressed direction over one still held from the last turn.
    let keys = [
        (KeyCode::Up, KeyCode::W, Direction::North),
        (KeyCode::Left, KeyCode::A, Direction::West),
        (KeyCode::Down, KeyCode::S, Direction::South),
        (KeyCode::Right, KeyCode::D, Direction::East),
    ];
    keys.iter()
        .find(|(arrow, letter, _)| is_key_pressed(*arrow) || is_key_pressed(*letter))
        .or_else(|| {
            keys.iter()
                .find(|(arrow, letter, _)| is_key_down(*arrow) || is_key_down(*letter))
        })
        .map(|(_, _, direction)| *direction)
}

pub(crate) fn window_fits() -> bool {
    // Macroquad already converts backing pixels into logical window sizes.
    screen_width() >= CANVAS_W && screen_height() >= CANVAS_H
}

#[derive(Default)]
pub(crate) struct Clock {
    elapsed: f32,
}

impl Clock {
    fn fraction(&self) -> f32 {
        self.elapsed / (f32::from(TICK_MS) / 1000.0)
    }
    fn reset(&mut self) {
        self.elapsed = 0.0;
    }

    pub(crate) fn advance(&mut self, game: &mut Game, seconds: f32, fits: bool) {
        if !fits || game.mode != Mode::Playing {
            self.reset();
            return;
        }
        let tick = f32::from(TICK_MS) / 1000.0;
        // Ignore time spent suspended or stalled: never burst queued damage.
        self.elapsed += seconds.clamp(0.0, tick);
        if self.elapsed + f32::EPSILON >= tick {
            game.tick();
            self.elapsed = (self.elapsed - tick).max(0.0);
        }
    }
}

// Whole-number enlargement and letterboxing keep every source pixel crisp.
// A window smaller than the logical canvas shows a resize message instead.
pub(crate) fn viewport(width: f32, height: f32) -> (f32, f32, f32) {
    let scale = (width / CANVAS_W).min(height / CANVAS_H).floor().max(1.0);
    (
        ((width - CANVAS_W * scale) / 2.0).floor(),
        ((height - CANVAS_H * scale) / 2.0).floor(),
        scale,
    )
}

pub(crate) struct Scene {
    atlas: Texture2D,
    canvas: RenderTarget,
    font: Font,
}

impl Scene {
    pub(crate) fn new() -> Result<Self, String> {
        let mut font = load_ttf_font_from_bytes(UI_FONT).map_err(|error| error.to_string())?;
        font.set_filter(FilterMode::Linear);
        let atlas = Texture2D::from_file_with_format(ATLAS, Some(ImageFormat::Png));
        atlas.set_filter(FilterMode::Nearest);
        let canvas = render_target(CANVAS_W as u32, CANVAS_H as u32);
        canvas.texture.set_filter(FilterMode::Nearest);
        Ok(Self {
            atlas,
            canvas,
            font,
        })
    }

    fn sprite(&self, sprite: Sprite, x: f32, y: f32, tint: Color) {
        let index = sprite as u16;
        draw_texture_ex(
            &self.atlas,
            x,
            y,
            tint,
            DrawTextureParams {
                source: Some(Rect::new(
                    f32::from(index % 8) * TILE,
                    f32::from(index / 8) * TILE,
                    TILE,
                    TILE,
                )),
                dest_size: Some(vec2(TILE, TILE)),
                ..Default::default()
            },
        );
    }

    fn at(&self, sprite: Sprite, pos: Pos, tint: Color) {
        self.sprite(
            sprite,
            MAP_X + f32::from(pos.x) * TILE,
            MAP_Y + f32::from(pos.y) * TILE,
            tint,
        );
    }

    fn actor(&self, sprite: Sprite, pos: Pos, tint: Color) {
        let x = MAP_X + f32::from(pos.x) * TILE;
        let y = MAP_Y + f32::from(pos.y) * TILE;
        draw_ellipse(x + 8.0, y + 14.0, 6.0, 2.0, 0.0, color_u8!(15, 29, 30, 90));
        self.sprite(sprite, x, y, tint);
    }

    pub(crate) fn draw(&self, game: &Game, audio: &str, fits: bool, fraction: f32) {
        set_camera(&Camera2D {
            render_target: Some(self.canvas.clone()),
            ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, CANVAS_W, CANVAS_H))
        });
        clear_background(INK);
        self.header(game);
        self.map(game, fraction);
        draw_rectangle(16.0, 370.0, 480.0, 1.0, PANEL);
        if overlay(game.mode).is_some() {
            draw_rectangle(
                MAP_X,
                MAP_Y,
                WIDTH as f32 * TILE,
                HEIGHT as f32 * TILE,
                color_u8!(15, 25, 29, 180),
            );
            draw_rectangle(51.0, 141.0, 410.0, 100.0, INK);
            draw_rectangle_lines(51.0, 141.0, 410.0, 100.0, 1.0, GOLD);
        }
        set_default_camera();
        clear_background(INK);
        if fits {
            let (x, y, scale) = viewport(screen_width(), screen_height());
            draw_texture_ex(
                &self.canvas.texture,
                x,
                y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(CANVAS_W * scale, CANVAS_H * scale)),
                    flip_y: true,
                    ..Default::default()
                },
            );
            // Text bypasses the low-resolution sprite canvas. Rasterize glyphs
            // at their final display size, including the platform's DPI scale.
            let ui = UiText {
                font: &self.font,
                origin: vec2(x, y),
                scale,
            };
            self.labels(&ui, game, audio);
        } else {
            let ui = UiText {
                font: &self.font,
                origin: Vec2::ZERO,
                scale: 1.0,
            };
            ui.text(
                "Paused: resize to at least 512 x 400.",
                vec2(12.0, 28.0),
                16,
                PAPER,
            );
            ui.text("Q quits.", vec2(12.0, 51.0), 16, MUTED);
        }
    }

    fn header(&self, game: &Game) {
        draw_rectangle(0.0, 0.0, CANVAS_W, 56.0, PANEL);
        draw_rectangle(16.0, 54.0, 480.0, 1.0, color_u8!(88, 106, 89, 255));
        for i in 0..game.max_hearts {
            self.sprite(
                if i < game.hearts {
                    Sprite::Heart
                } else if i == game.hearts && game.half_heart {
                    Sprite::HalfHeart
                } else {
                    Sprite::EmptyHeart
                },
                496.0 - f32::from(game.max_hearts) * 19.0 + f32::from(i) * 19.0,
                10.0,
                WHITE,
            );
        }
        if game.sword {
            self.sprite(
                if game.frost_sword {
                    Sprite::FrostSword
                } else {
                    Sprite::Sword
                },
                366.0,
                31.0,
                WHITE,
            );
        }
        if game.armor {
            self.sprite(Sprite::Armor, 344.0, 31.0, WHITE);
        }
    }

    fn map(&self, game: &Game, fraction: f32) {
        let beat = game.animation_ms / 300;
        let tint = match game.room {
            Room::Woods => color_u8!(205, 225, 209, 255),
            Room::Cave => color_u8!(205, 190, 178, 255),
            Room::Secret => color_u8!(223, 220, 204, 255),
            _ => WHITE,
        };
        draw_rectangle(
            MAP_X - 2.0,
            MAP_Y - 2.0,
            484.0,
            260.0,
            color_u8!(82, 101, 83, 255),
        );
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let pos = Pos::new(x, y);
                let glyph = world::tile(game.room, pos);
                let floor = floor_sprite(game.room, pos);
                self.at(floor, pos, tint);
                match glyph {
                    '#' => self.at(
                        match game.room {
                            Room::Cave | Room::Secret => Sprite::Wall,
                            Room::Desert => Sprite::DesertRock,
                            Room::Snow => Sprite::SnowRock,
                            _ => Sprite::Tree,
                        },
                        pos,
                        tint,
                    ),
                    'T' => {
                        // A small offset shadow adds depth without obscuring collision tiles.
                        draw_rectangle(
                            MAP_X + x as f32 * TILE + 3.0,
                            MAP_Y + y as f32 * TILE + 10.0,
                            13.0,
                            6.0,
                            color_u8!(20, 37, 29, 95),
                        );
                        self.at(
                            match game.room {
                                Room::Desert => Sprite::Cactus,
                                Room::Snow => Sprite::SnowPine,
                                _ if (x + y) % 3 == 0 => Sprite::TreeAlt,
                                _ => Sprite::Tree,
                            },
                            pos,
                            tint,
                        );
                    }
                    '~' => {
                        if game.room == Room::Snow {
                            self.at(Sprite::Ice, pos, WHITE);
                            continue;
                        }
                        self.at(
                            if (beat + x as u16) % 2 == 0 {
                                Sprite::Water
                            } else {
                                Sprite::WaterAlt
                            },
                            pos,
                            WHITE,
                        );
                        // Shore highlights are visual only: swimming remains duck business.
                        for direction in [
                            Direction::North,
                            Direction::West,
                            Direction::South,
                            Direction::East,
                        ] {
                            if world::walkable(game.room, pos.step(direction)) {
                                let px = MAP_X + x as f32 * TILE;
                                let py = MAP_Y + y as f32 * TILE;
                                let (sx, sy, w, h) = match direction {
                                    Direction::North => (px, py, TILE, 1.0),
                                    Direction::South => (px, py + 15.0, TILE, 1.0),
                                    Direction::West => (px, py, 1.0, TILE),
                                    Direction::East => (px + 15.0, py, 1.0, TILE),
                                };
                                draw_rectangle(sx, sy, w, h, color_u8!(156, 179, 122, 255));
                            }
                        }
                    }
                    'H' => self.actor(Sprite::Hermit, pos, WHITE),
                    '.' if !matches!(
                        game.room,
                        Room::Cave | Room::Secret | Room::Desert | Room::Snow
                    ) && !is_path(game.room, pos) =>
                    {
                        if (x * 7 + y * 13) % 31 == 0 {
                            self.at(Sprite::Flowers, pos, WHITE);
                        } else if world::tile(game.room, pos.step(Direction::South)) == '~' {
                            self.at(Sprite::Reeds, pos, WHITE);
                        }
                    }
                    _ => {}
                }
            }
        }
        if game.room == Room::Cave && !game.sword {
            glow(SWORD, game.animation_ms);
            self.at(Sprite::Sword, SWORD, WHITE);
        }
        for (room, pos, owned, sprite) in [
            (Room::Desert, ARMOR, game.armor, Sprite::Armor),
            (
                Room::Snow,
                FROST_SWORD,
                game.frost_sword,
                Sprite::FrostSword,
            ),
        ] {
            if game.room == room && !owned {
                glow(pos, game.animation_ms);
                self.at(sprite, pos, WHITE);
            }
        }
        if game.room == Room::Secret {
            let unlocked = game.actors().iter().all(|e| !e.alive);
            if unlocked {
                glow(CHEST, game.animation_ms);
            }
            self.at(
                Sprite::Chest,
                CHEST,
                if unlocked {
                    WHITE
                } else {
                    color_u8!(155, 155, 155, 255)
                },
            );
        }
        for &pos in game.containers() {
            glow(pos, game.animation_ms);
            self.at(Sprite::HeartContainer, pos, WHITE);
        }
        for critter in game.wildlife() {
            let sprite = critter_sprite(critter.kind, beat % 2 == 1);
            self.actor(sprite, critter.pos, WHITE);
        }
        for enemy in game.actors().iter().filter(|e| e.alive) {
            let sprite = match (enemy.kind, beat % 2 == 1) {
                (Kind::Slime, false) => Sprite::Slime0,
                (Kind::Slime, true) => Sprite::Slime1,
                (Kind::Beetle, false) => Sprite::Beetle0,
                (Kind::Beetle, true) => Sprite::Beetle1,
            };
            self.actor(
                sprite,
                enemy.pos,
                match game.room {
                    Room::Snow => color_u8!(125, 218, 255, 255),
                    Room::Desert => color_u8!(255, 204, 144, 255),
                    _ => WHITE,
                },
            );
            if enemy.health > 1 {
                let x = MAP_X + f32::from(enemy.pos.x) * TILE;
                let y = MAP_Y + f32::from(enemy.pos.y) * TILE;
                draw_rectangle(x + 4.0, y - 1.0, 3.0, 1.0, GOLD);
                draw_rectangle(x + 9.0, y - 1.0, 3.0, 1.0, GOLD);
            }
        }
        if let Some(pos) = game.impact.filter(|_| game.impact_ms > 0) {
            let x = MAP_X + f32::from(pos.x) * TILE + 8.0;
            let y = MAP_Y + f32::from(pos.y) * TILE + 8.0;
            let spread = 2.0 + f32::from(200 - game.impact_ms) / 25.0;
            for (dx, dy) in [(-1.0, -0.5), (1.0, -1.0), (-0.5, 1.0), (1.0, 0.5)] {
                draw_rectangle(
                    (x + dx * spread).round(),
                    (y + dy * spread).round(),
                    2.0,
                    2.0,
                    GOLD,
                );
            }
        }
        let hero = hero_sprite(game.facing, game.walking() && beat % 2 == 1);
        let hero_tint = if game.immune_ms > 0 && (game.immune_ms / 100) % 2 == 0 {
            color_u8!(180, 215, 221, 180)
        } else {
            WHITE
        };
        self.actor(hero, game.player, hero_tint);
        if game.slash_ms > 0 {
            self.slash(game, fraction);
        }
        // Quiet motes in the woods. Use simulation time so pause freezes them.
        if game.room == Room::Woods {
            for i in 0..9 {
                let x = MAP_X + 40.0 + ((i * 47) % 400) as f32;
                let y = MAP_Y + 32.0 + ((i * 37) % 200) as f32;
                let drift = ((game.animation_ms as f32 / 380.0) + i as f32).sin() * 2.0;
                draw_rectangle(
                    x,
                    (y + drift).round(),
                    1.0,
                    1.0,
                    color_u8!(236, 218, 126, 140),
                );
            }
        }
    }

    fn slash(&self, game: &Game, fraction: f32) {
        let reach = game.attack_tiles().count();
        if reach == 0 {
            return;
        }
        let progress = (1.0
            - (f32::from(game.slash_ms) - fraction * f32::from(TICK_MS)) / f32::from(SLASH_MS))
        .clamp(0.0, 1.0);
        let forward = facing_angle(game.facing);
        let angle = forward - 0.65 + progress * 1.3;
        // Match the arm pixels in each facing sprite, rather than anchoring
        // north/south swings to the head or the hero's feet.
        let grip = match game.facing {
            Direction::North => vec2(11.0, 8.0),
            Direction::South => vec2(11.0, 10.0),
            Direction::East => vec2(12.0, 10.0),
            Direction::West => vec2(3.0, 10.0),
        };
        let hand = vec2(
            MAP_X + f32::from(game.player.x) * TILE,
            MAP_Y + f32::from(game.player.y) * TILE,
        ) + grip;
        let sprite = if game.frost_sword {
            Sprite::FrostBlade
        } else {
            Sprite::Blade
        };
        let length = TILE * reach as f32;
        let tip_radius = length - 2.0;
        let color = if game.frost_sword {
            color_u8!(147, 225, 244, 140)
        } else {
            color_u8!(250, 241, 208, 140)
        };
        // A short fading crescent follows the tip, not a detached tile marker.
        for i in 0..5 {
            let a = angle - i as f32 * 0.07;
            let b = a - 0.07;
            if b < forward - 0.65 {
                break;
            }
            let start = hand + vec2(a.cos(), a.sin()) * tip_radius;
            let end = hand + vec2(b.cos(), b.sin()) * tip_radius;
            draw_line(
                start.x.round(),
                start.y.round(),
                end.x.round(),
                end.y.round(),
                2.0,
                Color {
                    a: color.a * (1.0 - i as f32 / 5.0),
                    ..color
                },
            );
        }
        draw_texture_ex(
            &self.atlas,
            hand.x - 8.0,
            hand.y - length + 2.0 * reach as f32,
            WHITE,
            DrawTextureParams {
                source: Some(Rect::new(
                    (sprite as u16 % 8) as f32 * TILE,
                    (sprite as u16 / 8) as f32 * TILE,
                    TILE,
                    TILE,
                )),
                dest_size: Some(vec2(TILE, length)),
                pivot: Some(hand),
                rotation: angle + std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
        );
    }

    fn labels(&self, ui: &UiText<'_>, game: &Game, audio: &str) {
        ui.text("THE BORROWED SWORD", vec2(16.0, 26.0), 22, GOLD);
        ui.text(game.room.name(), vec2(17.0, 46.0), 14, MUTED);
        ui.text(
            &format!("SOUND: {}", audio.to_uppercase()),
            vec2(390.0, 44.0),
            11,
            MUTED,
        );
        let message = game.message();
        ui.text(message[0], vec2(18.0, 342.0), 14, PAPER);
        ui.text(message[1], vec2(18.0, 361.0), 14, MUTED);
        ui.text(
            "WASD / ARROWS  move    SPACE  sword    E  greet",
            vec2(18.0, 382.0),
            12,
            GOLD,
        );
        ui.text(
            "P  pause    M  sound    R  restart    Q / ESC  quit",
            vec2(18.0, 396.0),
            12,
            MUTED,
        );
        if let Some((title, body, controls)) = overlay(game.mode) {
            ui.centered(title, 170.0, 20, GOLD);
            ui.centered(body, 196.0, 14, PAPER);
            ui.centered(controls, 222.0, 14, MUTED);
        }
    }
}

struct UiText<'a> {
    font: &'a Font,
    origin: Vec2,
    scale: f32,
}

fn facing_angle(direction: Direction) -> f32 {
    match direction {
        Direction::North => -std::f32::consts::FRAC_PI_2,
        Direction::East => 0.0,
        Direction::South => std::f32::consts::FRAC_PI_2,
        Direction::West => std::f32::consts::PI,
    }
}

impl UiText<'_> {
    fn text(&self, value: &str, pos: Vec2, size: u16, color: Color) {
        let pos = self.origin + pos * self.scale;
        draw_text_ex(
            value,
            pos.x,
            pos.y,
            TextParams {
                font: Some(self.font),
                font_size: (f32::from(size) * self.scale) as u16,
                color,
                ..Default::default()
            },
        );
    }

    fn centered(&self, value: &str, y: f32, size: u16, color: Color) {
        let size_on_screen = (f32::from(size) * self.scale) as u16;
        let width = measure_text(value, Some(self.font), size_on_screen, 1.0).width;
        let x = (CANVAS_W - width / self.scale) / 2.0;
        self.text(value, vec2(x, y), size, color);
    }
}

fn glow(pos: Pos, time: u16) {
    let x = MAP_X + f32::from(pos.x) * TILE;
    let y = MAP_Y + f32::from(pos.y) * TILE;
    draw_rectangle(x - 2.0, y - 2.0, 20.0, 20.0, color_u8!(249, 210, 111, 25));
    let shift = f32::from((time / 300) % 3);
    draw_rectangle(x + 2.0 + shift * 4.0, y - 2.0, 1.0, 3.0, GOLD);
    draw_rectangle(x + 1.0 + shift * 4.0, y - 1.0, 3.0, 1.0, GOLD);
}

fn is_path(room: Room, pos: Pos) -> bool {
    match room {
        Room::Glade => {
            (14..=16).contains(&pos.x) && pos.y <= 10 || pos.x >= 15 && (7..=9).contains(&pos.y)
        }
        Room::Clearing => (7..=9).contains(&pos.y),
        Room::Woods => (14..=16).contains(&pos.x) || (7..=9).contains(&pos.y),
        Room::Secret | Room::Cave => false,
        Room::Desert | Room::Snow => false,
    }
}

fn floor_sprite(room: Room, pos: Pos) -> Sprite {
    match room {
        Room::Cave => Sprite::CaveFloor,
        Room::Secret => Sprite::StoneFloor,
        Room::Desert if (pos.x + pos.y) % 2 == 0 => Sprite::Sand,
        Room::Desert => Sprite::SandAlt,
        Room::Snow if (pos.x + pos.y) % 2 == 0 => Sprite::Snow,
        Room::Snow => Sprite::SnowAlt,
        _ if is_path(room, pos) => Sprite::Path,
        Room::Woods => Sprite::WoodsFloor,
        _ if (pos.x + pos.y) % 2 == 0 => Sprite::Grass,
        _ => Sprite::GrassAlt,
    }
}

fn critter_sprite(kind: CritterKind, frame: bool) -> Sprite {
    match (kind, frame) {
        (CritterKind::Rabbit, false) => Sprite::Rabbit0,
        (CritterKind::Rabbit, true) => Sprite::Rabbit1,
        (CritterKind::Duck, false) => Sprite::Duck0,
        (CritterKind::Duck, true) => Sprite::Duck1,
        (CritterKind::Tortoise, false) => Sprite::Tortoise0,
        (CritterKind::Tortoise, true) => Sprite::Tortoise1,
    }
}

fn hero_sprite(direction: Direction, frame: bool) -> Sprite {
    match (direction, frame) {
        (Direction::South, false) => Sprite::HeroSouth0,
        (Direction::South, true) => Sprite::HeroSouth1,
        (Direction::North, false) => Sprite::HeroNorth0,
        (Direction::North, true) => Sprite::HeroNorth1,
        (Direction::East, false) => Sprite::HeroEast0,
        (Direction::East, true) => Sprite::HeroEast1,
        (Direction::West, false) => Sprite::HeroWest0,
        (Direction::West, true) => Sprite::HeroWest1,
    }
}

fn overlay(mode: Mode) -> Option<(&'static str, &'static str, &'static str)> {
    match mode {
        Mode::Playing => None,
        Mode::Paused => Some((
            "PAUSED",
            "Even heroes need a compile break.",
            "P to resume  /  Q to quit",
        )),
        Mode::Dead => Some((
            "YOU HAVE BEEN DEFEATED",
            "By an extremely unqualified opponent.",
            "R for a fresh adventure  /  Q to quit",
        )),
        Mode::Won => Some((
            "UNREASONABLE CONFIDENCE",
            "Your certificate is well earned. Probably.",
            "R to play again  /  Q to quit",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_scaling_centers_without_cropping() {
        for (w, h, expected) in [
            (512.0, 400.0, 1.0),
            (1024.0, 800.0, 2.0),
            (1400.0, 900.0, 2.0),
            (1401.0, 901.0, 2.0),
            (1920.0, 1200.0, 3.0),
        ] {
            let (x, y, scale) = viewport(w, h);
            assert_eq!(scale, expected);
            assert_eq!(x.fract(), 0.0);
            assert_eq!(y.fract(), 0.0);
            assert!(x >= 0.0 && y >= 0.0);
            assert!(x + CANVAS_W * scale <= w);
            assert!(y + CANVAS_H * scale <= h);
        }
    }

    #[test]
    fn clock_is_frame_rate_independent_and_discards_stalls() {
        for frames_per_tick in [1, 3, 5] {
            let mut game = Game::new();
            let mut clock = Clock::default();
            for _ in 0..frames_per_tick * 10 {
                clock.advance(&mut game, 0.05 / frames_per_tick as f32, true);
            }
            assert_eq!(game.animation_ms, 500);
        }
        let mut game = Game::new();
        let mut clock = Clock::default();
        clock.advance(&mut game, 30.0, true);
        assert_eq!(game.animation_ms, TICK_MS);
        clock.advance(&mut game, 30.0, false);
        game.act(Action::Pause);
        clock.advance(&mut game, 30.0, true);
        assert_eq!(game.animation_ms, TICK_MS);
    }

    #[test]
    fn embedded_atlas_decodes_with_every_sprite_inside_bounds() {
        let image = Image::from_file_with_format(ATLAS, Some(ImageFormat::Png)).unwrap();
        assert_eq!(image.width, 128);
        assert_eq!(image.height, 112);
        for sprite in [
            Sprite::HeroSouth0,
            Sprite::Rabbit0,
            Sprite::Duck0,
            Sprite::Tortoise0,
            Sprite::Slime0,
            Sprite::Beetle0,
            Sprite::Heart,
        ] {
            let index = sprite as usize;
            let x = (index % 8) * 16;
            let y = (index / 8) * 16;
            assert!(x + 16 <= image.width as usize && y + 16 <= image.height as usize);
            let colors: Vec<_> = (y..y + 16)
                .flat_map(|py| (x..x + 16).map(move |px| (py * 128 + px) * 4))
                .map(|offset| &image.bytes[offset..offset + 4])
                .collect();
            assert!(
                colors.iter().any(|c| c[3] == 0),
                "actors need transparent silhouettes"
            );
            assert!(colors.iter().any(|c| c[3] == 255), "sprite must be visible");
        }
    }
}
