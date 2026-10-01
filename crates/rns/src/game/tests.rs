use super::*;
use std::collections::VecDeque;

fn ticks(game: &mut Game, count: usize) {
    for _ in 0..count {
        game.tick();
    }
}

fn exit(game: &mut Game, direction: Direction) {
    game.player = direction.doorway();
    game.move_ms = 0;
    game.slash_ms = 0;
    game.act(Action::Move(direction));
}

#[test]
fn movement_is_throttled_and_walls_change_facing() {
    let mut game = Game::new();
    game.act(Action::Move(Direction::East));
    assert_eq!(game.player, Pos::new(16, 10));
    game.act(Action::Move(Direction::East));
    assert_eq!(game.player, Pos::new(16, 10));
    ticks(&mut game, 3);
    game.act(Action::Move(Direction::East));
    assert_eq!(game.player, Pos::new(17, 10));
    ticks(&mut game, 3);
    game.player = Pos::new(3, 6);
    game.act(Action::Move(Direction::North));
    assert_eq!(game.player, Pos::new(3, 6));
    assert_eq!(game.facing, Direction::North);
}

#[test]
fn cave_equips_sword_only_once_and_hermit_blocks_movement() {
    let mut game = Game::new();
    game.act(Action::Attack);
    assert_eq!(game.slash_ms, 0);
    assert!(game.message()[0].contains("cave"));
    exit(&mut game, Direction::North);
    assert_eq!(game.room, Room::Cave);
    assert_eq!(game.player, Direction::South.entrance());
    game.player = SWORD.step(Direction::South);
    game.move_ms = 0;
    game.act(Action::Move(Direction::North));
    assert!(game.sword);
    assert!(game.message()[0].contains("dependency"));
    ticks(&mut game, 71);
    assert_eq!(game.message(), Room::Cave.description());
    game.player = world::HERMIT.step(Direction::South);
    game.act(Action::Move(Direction::North));
    assert_ne!(game.player, world::HERMIT);
}

#[test]
fn sword_hits_only_adjacent_facing_tile_and_locks_facing() {
    for direction in [
        Direction::North,
        Direction::West,
        Direction::South,
        Direction::East,
    ] {
        let mut game = Game::new();
        game.room = Room::Clearing;
        game.player = Pos::new(15, 8);
        game.facing = direction;
        game.sword = true;
        let adjacent = game.player.step(direction);
        let distant = adjacent.step(direction);
        let behind = game.player.step(direction.opposite());
        game.enemies[game.room.index()] = vec![
            Enemy::new(adjacent.x, adjacent.y, Kind::Slime),
            Enemy::new(distant.x, distant.y, Kind::Slime),
            Enemy::new(behind.x, behind.y, Kind::Slime),
        ];
        game.act(Action::Attack);
        assert!(!game.actors()[0].alive);
        assert!(game.actors()[1].alive);
        assert!(game.actors()[2].alive);
        game.act(Action::Move(direction.opposite()));
        assert_eq!(game.facing, direction);
        assert_eq!(game.player, Pos::new(15, 8));
        ticks(&mut game, 3);
        assert_eq!(game.slash_ms, 0);
        game.act(Action::Attack);
        assert_eq!(game.slash_ms, 0);
        ticks(&mut game, 3);
        game.act(Action::Attack);
        assert_eq!(game.slash_ms, SLASH_MS);
    }
}

#[test]
fn an_enemy_walking_into_an_active_slash_dies() {
    let mut game = Game::new();
    game.room = Room::Clearing;
    game.player = Pos::new(15, 8);
    game.facing = Direction::West;
    game.sword = true;
    game.enemies[game.room.index()] = vec![Enemy::new(13, 8, Kind::Slime)];
    game.enemy_ms = TICK_MS;
    game.act(Action::Attack);
    assert!(game.actors()[0].alive);
    game.tick();
    assert!(!game.actors()[0].alive);
}

#[test]
fn contact_damages_knocks_back_and_grants_immunity() {
    let mut game = Game::new();
    game.room = Room::Clearing;
    game.player = Pos::new(11, 8);
    game.act(Action::Move(Direction::West));
    assert_eq!(game.hearts, 2);
    assert_eq!(game.player, Pos::new(11, 8));
    assert_eq!(game.immune_ms, IMMUNE_MS);
    ticks(&mut game, 3);
    game.act(Action::Move(Direction::West));
    assert_eq!(game.hearts, 2);
    assert_eq!(game.player, Pos::new(10, 8));
    game.immune_ms = TICK_MS;
    game.tick();
    assert_eq!(game.hearts, 1);
}

#[test]
fn blocked_knockback_stays_in_place_and_death_freezes_game() {
    let mut game = Game::new();
    game.room = Room::Clearing;
    game.player = Pos::new(10, 4);
    game.facing = Direction::East;
    game.hearts = 1;
    game.enemies[game.room.index()] = vec![Enemy::new(10, 4, Kind::Slime)];
    game.tick();
    assert_eq!(game.hearts, 0);
    assert_eq!(game.player, Pos::new(10, 4));
    assert_eq!(game.mode, Mode::Dead);
    let immune = game.immune_ms;
    ticks(&mut game, 50);
    game.act(Action::Move(Direction::South));
    assert_eq!(game.player, Pos::new(10, 4));
    assert_eq!(game.immune_ms, immune);
    game.act(Action::Restart);
    assert_eq!(game.room, Room::Glade);
    assert_eq!(game.player, START);
    assert_eq!(game.hearts, 3);
    assert!(!game.sword);
    assert_eq!(game.mode, Mode::Playing);
    assert!(game.enemies.iter().flatten().all(|e| e.alive));
}

#[test]
fn pause_freezes_timers_enemies_and_input() {
    let mut game = Game::new();
    game.room = Room::Clearing;
    game.sword = true;
    game.act(Action::Attack);
    game.act(Action::Pause);
    let player = game.player;
    let enemy = game.actors()[0].pos;
    ticks(&mut game, 100);
    game.act(Action::Move(Direction::East));
    game.act(Action::Restart);
    assert_eq!(game.mode, Mode::Paused);
    assert_eq!(game.slash_ms, SLASH_MS);
    assert_eq!(game.player, player);
    assert_eq!(game.actors()[0].pos, enemy);
    game.act(Action::Pause);
    game.tick();
    assert_eq!(game.slash_ms, SLASH_MS - TICK_MS);
}

#[test]
fn exact_route_reveals_secret_and_only_edge_crossings_count() {
    let mut game = Game::new();
    game.room = Room::Woods;
    for direction in ROUTE {
        game.player = Pos::new(15, 8);
        game.move_ms = 0;
        game.act(Action::Move(direction));
        assert_eq!(game.woods_progress, 0);
    }
    for (i, direction) in ROUTE.into_iter().enumerate() {
        exit(&mut game, direction);
        if i < 3 {
            assert_eq!(game.room, Room::Woods);
            assert_eq!(game.woods_progress, i + 1);
            assert_eq!(game.player, direction.opposite().entrance());
        }
    }
    assert_eq!(game.room, Room::Secret);
    assert_eq!(game.player, Direction::East.entrance());
    exit(&mut game, Direction::East);
    assert_eq!(game.room, Room::Woods);
    assert_eq!(game.woods_progress, 0);
}

#[test]
fn every_wrong_turn_resets_without_reusing_the_mistake() {
    for progress in 0..ROUTE.len() {
        for direction in [Direction::North, Direction::West, Direction::South] {
            if direction == ROUTE[progress] {
                continue;
            }
            let mut game = Game::new();
            game.room = Room::Woods;
            for step in &ROUTE[..progress] {
                exit(&mut game, *step);
            }
            exit(&mut game, direction);
            assert_eq!(game.room, Room::Woods);
            assert_eq!(game.woods_progress, 0);
        }
    }
}

#[test]
fn east_always_escapes_and_reentry_starts_fresh() {
    for progress in 0..ROUTE.len() {
        let mut game = Game::new();
        game.room = Room::Woods;
        for direction in &ROUTE[..progress] {
            exit(&mut game, *direction);
        }
        exit(&mut game, Direction::East);
        assert_eq!(game.room, Room::Clearing);
        assert_eq!(game.player, Direction::East.entrance());
        assert_eq!(game.woods_progress, 0);
        exit(&mut game, Direction::East);
        assert_eq!(game.room, Room::Woods);
        assert_eq!(game.woods_progress, 0);
    }
}

#[test]
fn transitions_preserve_kills_positions_and_clear_arrival_tile() {
    let mut game = Game::new();
    game.room = Room::Woods;
    let index = game.room.index();
    game.enemies[index][0].alive = false;
    let position = game.enemies[index][1].pos;
    exit(&mut game, Direction::North);
    assert!(!game.actors()[0].alive);
    assert_eq!(game.actors()[1].pos, position);
    assert_eq!(game.immune_ms, IMMUNE_MS);
    game.enemies[index][1].pos = Direction::East.entrance();
    exit(&mut game, Direction::West);
    assert!(
        game.actors()
            .iter()
            .all(|e| !e.alive || e.pos != game.player)
    );
    assert!(!game.actors()[0].alive);
    let relocated = game.actors()[1].pos;
    game.move_enemies();
    assert_ne!(game.actors()[1].pos, relocated);
    exit(&mut game, Direction::East);
    exit(&mut game, Direction::East);
    assert!(!game.actors()[0].alive);
}

#[test]
fn treasure_requires_defeated_guards_and_victory_allows_replay() {
    let mut game = Game::new();
    game.room = Room::Secret;
    game.player = CHEST;
    game.tick();
    assert_eq!(game.mode, Mode::Playing);
    assert!(game.message()[0].contains("guarded"));
    for enemy in &mut game.enemies[Room::Secret.index()] {
        enemy.alive = false;
    }
    game.tick();
    assert_eq!(game.mode, Mode::Won);
    let player = game.player;
    game.act(Action::Move(Direction::South));
    assert_eq!(game.player, player);
    game.act(Action::Restart);
    assert_eq!(game.mode, Mode::Playing);
    assert_eq!(game.room, Room::Glade);
}

#[test]
fn enemies_follow_repeatable_safe_patterns_and_do_not_pursue() {
    for room in [Room::Clearing, Room::Woods, Room::Secret] {
        let mut first = Game::new();
        let mut second = Game::new();
        first.room = room;
        second.room = room;
        first.player = Pos::new(1, 1);
        second.player = Pos::new(28, 14);
        for _ in 0..200 {
            first.move_enemies();
            second.move_enemies();
            for (a, b) in first.actors().iter().zip(second.actors()) {
                assert_eq!(a.pos, b.pos);
                assert!(world::walkable(room, a.pos));
                assert!(a.pos.x > 0 && a.pos.x < world::WIDTH - 1);
                assert!(a.pos.y > 0 && a.pos.y < world::HEIGHT - 1);
                if a.kind == Kind::Beetle {
                    assert!((a.pos.x - a.origin.x).abs() <= 3);
                }
                assert_eq!(first.actors().iter().filter(|e| e.pos == a.pos).count(), 1);
            }
        }
    }
}

#[test]
fn every_map_item_exit_and_enemy_is_reachable() {
    let game = Game::new();
    for room in Room::ALL {
        let start = room.exits()[0].entrance();
        let mut reached = vec![start];
        let mut queue = VecDeque::from([start]);
        while let Some(pos) = queue.pop_front() {
            for direction in [
                Direction::North,
                Direction::West,
                Direction::South,
                Direction::East,
            ] {
                let next = pos.step(direction);
                if world::walkable(room, next) && !reached.contains(&next) {
                    reached.push(next);
                    queue.push_back(next);
                }
            }
        }
        for direction in room.exits() {
            assert!(
                reached.contains(&direction.doorway()),
                "{room:?}: {direction:?}"
            );
            assert!(reached.contains(&direction.entrance()));
        }
        for enemy in &game.enemies[room.index()] {
            assert!(reached.contains(&enemy.pos));
        }
        let item = match room {
            Room::Glade => START,
            Room::Cave => SWORD,
            Room::Secret => CHEST,
            _ => start,
        };
        assert!(reached.contains(&item));
        for y in 0..world::HEIGHT {
            for x in 0..world::WIDTH {
                let pos = Pos::new(x, y);
                assert!(world::tile(room, pos).is_ascii());
                if x == 0 || y == 0 || x == world::WIDTH - 1 || y == world::HEIGHT - 1 {
                    assert_eq!(
                        world::walkable(room, pos),
                        room.exits().iter().any(|d| d.doorway() == pos)
                    );
                }
            }
        }
    }
}

#[test]
fn ordinary_connections_return_to_valid_doorways() {
    let mut game = Game::new();
    for (direction, room, entrance) in [
        (Direction::North, Room::Cave, Direction::South),
        (Direction::South, Room::Glade, Direction::North),
        (Direction::East, Room::Clearing, Direction::West),
        (Direction::West, Room::Glade, Direction::East),
    ] {
        exit(&mut game, direction);
        assert_eq!(game.room, room);
        assert_eq!(game.player, entrance.entrance());
        assert!(world::walkable(room, game.player));
    }
}

#[test]
fn attack_feedback_fires_once_for_real_swings_and_hits() {
    let mut game = Game::new();
    game.act(Action::Attack);
    assert!(game.drain_sounds().next().is_none());
    game.sword = true;
    game.room = Room::Clearing;
    game.player = Pos::new(9, 8);
    game.facing = Direction::East;
    game.act(Action::Attack);
    assert_eq!(
        game.drain_sounds().collect::<Vec<_>>(),
        [SoundCue::Sword, SoundCue::Hit]
    );
    ticks(&mut game, 3);
    game.act(Action::Attack);
    assert!(game.drain_sounds().next().is_none());
    ticks(&mut game, 3);
    game.act(Action::Attack);
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Sword]);
    game.act(Action::Pause);
    ticks(&mut game, 20);
    game.act(Action::Attack);
    assert!(game.drain_sounds().next().is_none());
}

#[test]
fn milestone_sounds_do_not_repeat_and_restart_clears_old_cues() {
    let mut game = Game::new();
    game.room = Room::Cave;
    game.player = SWORD;
    ticks(&mut game, 10);
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Pickup]);
    ticks(&mut game, 10);
    assert!(game.drain_sounds().next().is_none());
    game.room = Room::Woods;
    for direction in ROUTE {
        exit(&mut game, direction);
    }
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Secret]);
    for enemy in &mut game.enemies[Room::Secret.index()] {
        enemy.alive = false;
    }
    game.player = CHEST;
    ticks(&mut game, 10);
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Victory]);
    game.sounds.push(SoundCue::Sword);
    game.act(Action::Restart);
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Restart]);
}

#[test]
fn damage_feedback_respects_invulnerability_and_death_is_one_shot() {
    let mut game = Game::new();
    game.room = Room::Clearing;
    game.player = Pos::new(10, 8);
    game.hearts = 2;
    game.tick();
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Hurt]);
    game.player = Pos::new(10, 8);
    game.tick();
    assert!(game.drain_sounds().next().is_none());
    game.immune_ms = 0;
    ticks(&mut game, 10);
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Death]);
    assert_eq!(game.mode, Mode::Dead);
}
