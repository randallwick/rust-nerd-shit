//! Capture real GPU-rendered frames without controlling other applications.
//! cargo run -p rns --example graphics_smoke -- /tmp/rns-sprite-qa
#![allow(dead_code)] // Reuse production modules; this harness exercises rendering.
#[path = "../src/audio.rs"]
mod audio;
#[path = "../src/game.rs"]
mod game;
#[path = "../src/graphical.rs"]
mod graphical;
#[path = "../src/sprites.rs"]
mod sprites;
#[path = "../src/world.rs"]
mod world;

use game::{Action, Game, Mode};
use macroquad::prelude::*;
use std::path::PathBuf;
use world::{Direction, Pos, Room};

fn main() {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: graphics_smoke <output-directory>");
        std::process::exit(2);
    };
    if let Err(error) = std::fs::create_dir_all(&path) {
        eprintln!("cannot create {}: {error}", path.display());
        std::process::exit(1);
    }
    macroquad::Window::from_config(graphical::config(), capture(path));
}

async fn save(scene: &graphical::Scene, game: &Game, path: &std::path::Path, name: &str) {
    scene.draw(game, "muted", true, 0.0);
    next_frame().await;
    scene.draw(game, "muted", true, 0.0);
    let image = get_screen_data();
    assert!(image.bytes.chunks_exact(4).any(|p| p[0] > 150));
    image.export_png(&path.join(format!("{name}.png")).to_string_lossy());
}

async fn capture(path: PathBuf) {
    let scene = graphical::Scene::new().expect("embedded graphics assets load");
    let mut game = Game::new();
    next_frame().await;
    save(&scene, &game, &path, "glade").await;
    let original = game.wildlife()[0].pos;
    game.player = original.step(Direction::South);
    game.act(Action::Greet);
    assert!(game.message()[0].contains("rabbit"));
    save(&scene, &game, &path, "greet").await;
    for _ in 0..18 {
        game.tick();
    }
    assert_ne!(game.wildlife()[0].pos, original);
    save(&scene, &game, &path, "glade-animated").await;
    for room in [
        Room::Cave,
        Room::Clearing,
        Room::Woods,
        Room::Secret,
        Room::Desert,
        Room::Snow,
    ] {
        let mut game = Game::new();
        game.room = room;
        game.player = room.exits()[0].entrance();
        save(&scene, &game, &path, &format!("{room:?}").to_lowercase()).await;
    }
    game.sword = true;
    game.facing = Direction::East;
    game.act(Action::Attack);
    save(&scene, &game, &path, "sword").await;
    // Capture each swing phase in all directions using actual render output.
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        for frost in [false, true] {
            let mut swing = Game::new();
            swing.sword = true;
            swing.frost_sword = frost;
            swing.player = Pos::new(15, 8);
            swing.facing = direction;
            swing.act(Action::Attack);
            for phase in 0..6 {
                scene.draw(&swing, "muted", true, (phase % 2) as f32 * 0.5);
                let image = get_screen_data();
                image.export_png(
                    &path
                        .join(format!("swing-{direction:?}-{frost}-{phase}.png"))
                        .to_string_lossy(),
                );
                if phase % 2 == 1 {
                    swing.tick();
                }
            }
        }
    }
    let mut equipped = Game::new();
    equipped.room = Room::Desert;
    equipped.player = world::ARMOR;
    equipped.tick();
    assert!(equipped.armor);
    equipped.room = Room::Snow;
    equipped.player = world::FROST_SWORD;
    equipped.tick();
    assert!(equipped.frost_sword && equipped.sword);
    save(&scene, &equipped, &path, "equipped").await;
    // Defeat the real designated droppers and collect through movement input.
    let mut health = Game::new();
    health.sword = true;
    health.frost_sword = true;
    for (index, room) in [Room::Clearing, Room::Desert, Room::Snow]
        .into_iter()
        .enumerate()
    {
        health.room = room;
        let target = health.actors()[0].pos;
        health.player = target.step(Direction::West);
        health.facing = Direction::East;
        health.act(Action::Attack);
        assert_eq!(health.containers(), &[target]);
        save(&scene, &health, &path, &format!("heart-drop-{index}")).await;
        for _ in 0..6 {
            health.tick();
        }
        health.act(Action::Move(Direction::East));
        assert!(health.containers().is_empty());
        assert_eq!(health.max_hearts, 4 + index as u8);
        assert_eq!(health.hearts, health.max_hearts);
        save(&scene, &health, &path, &format!("heart-pickup-{index}")).await;
    }
    health.room = Room::Glade;
    health.player = world::START;
    health.hearts = 2;
    health.half_heart = true;
    save(&scene, &health, &path, "six-heart-injured").await;
    for _ in 0..400 {
        health.tick();
    }
    assert_eq!(health.health_halves(), 6);
    save(&scene, &health, &path, "regenerated-half-heart").await;
    for _ in 0..2400 {
        health.tick();
    }
    assert_eq!(health.hearts, 6);
    assert!(!health.half_heart);
    save(&scene, &health, &path, "regenerated-full-health").await;
    for mode in [Mode::Paused, Mode::Dead, Mode::Won] {
        game.mode = mode;
        let time = game.animation_ms;
        let mut clock = graphical::Clock::default();
        clock.advance(&mut game, 30.0, true);
        assert_eq!(game.animation_ms, time);
        save(&scene, &game, &path, &format!("{mode:?}").to_lowercase()).await;
    }
    game.act(Action::Restart);
    assert_eq!(game.player, world::START);
    assert_eq!(game.wildlife(), Game::new().wildlife());
    save(&scene, &game, &path, "restart").await;
    // Capture 1x text as well as the default 2x scene. Native window resizing
    // includes the title bar on macOS, so allow enough height for the canvas.
    miniquad::window::set_window_size(512, 440);
    for _ in 0..60 {
        next_frame().await;
    }
    assert!(graphical::window_fits());
    save(&scene, &game, &path, "small-text").await;
    game.player = Pos::new(15, 10);
    miniquad::window::set_window_size(350, 250);
    for _ in 0..60 {
        next_frame().await;
        if !graphical::window_fits() {
            break;
        }
    }
    let mut clock = graphical::Clock::default();
    assert!(!graphical::window_fits());
    clock.advance(&mut game, 30.0, graphical::window_fits());
    assert_eq!(game.animation_ms, 0);
    scene.draw(&game, "muted", false, 0.0);
    get_screen_data().export_png(&path.join("undersized.png").to_string_lossy());
    miniquad::window::set_window_size(1024, 800);
    for _ in 0..60 {
        next_frame().await;
        if graphical::window_fits() {
            break;
        }
    }
    assert!(graphical::window_fits());
    save(&scene, &game, &path, "resized").await;
    println!(
        "Native sprite, animation, greet, sword, heart drops, regeneration, overlays, restart and resize smoke checks passed. Frames: {}",
        path.display()
    );
}
