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
    assert_eq!(game.impact, Some(Pos::new(14, 8)));
    assert_eq!(game.impact_ms, 200);
    ticks(&mut game, 4);
    assert_eq!(game.impact_ms, 0);
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
            Room::Desert => ARMOR,
            Room::Snow => FROST_SWORD,
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

#[test]
fn peaceful_wildlife_cannot_hurt_be_killed_or_block_a_path() {
    for kind in [
        CritterKind::Rabbit,
        CritterKind::Duck,
        CritterKind::Tortoise,
    ] {
        let mut game = Game::new();
        let pos = Pos::new(16, 10);
        game.critters[Room::Glade.index()] = vec![Critter::new(pos.x, pos.y, kind)];
        game.sword = true;
        game.facing = Direction::East;
        game.act(Action::Attack);
        assert_eq!(game.wildlife().len(), 1);
        assert_eq!(game.wildlife()[0].pos, pos);
        assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Sword]);
        ticks(&mut game, 3);
        game.act(Action::Move(Direction::East));
        assert_eq!(game.player, pos);
        assert_eq!(game.hearts, 3);
        assert_eq!(game.mode, Mode::Playing);
    }
}

#[test]
fn wildlife_wanders_repeatably_in_valid_habitats_without_overlapping() {
    for room in Room::ALL {
        let mut first = Game::new();
        let mut second = Game::new();
        first.room = room;
        second.room = room;
        let origins: Vec<_> = first.wildlife().iter().map(|c| c.pos).collect();
        let mut moved = vec![false; origins.len()];
        for _ in 0..200 {
            first.move_critters();
            second.move_critters();
            assert_eq!(first.wildlife(), second.wildlife());
            for (i, critter) in first.wildlife().iter().enumerate() {
                assert!(critter.habitat(room, critter.pos));
                assert_ne!(critter.pos, first.player);
                assert!(
                    !first
                        .actors()
                        .iter()
                        .any(|e| e.alive && e.pos == critter.pos)
                );
                assert_eq!(
                    first
                        .wildlife()
                        .iter()
                        .filter(|c| c.pos == critter.pos)
                        .count(),
                    1
                );
                moved[i] |= critter.pos != origins[i];
            }
        }
        assert!(
            moved.iter().all(|moved| *moved),
            "{room:?}: every animal should wander"
        );
    }
}

#[test]
fn wildlife_avoids_the_player_and_enemies_when_choosing_a_step() {
    let mut game = Game::new();
    let pos = game.wildlife()[0].pos;
    game.player = pos.step(Direction::East);
    game.move_critters();
    assert_eq!(game.wildlife()[0].pos, pos);
    game.player = START;
    game.critters[0][0].phase = 0;
    let occupied = pos.step(Direction::East);
    game.enemies[0].push(Enemy::new(occupied.x, occupied.y, Kind::Slime));
    game.move_critters();
    assert_eq!(game.wildlife()[0].pos, pos);
}

#[test]
fn greet_uses_nearest_animal_within_two_tiles_and_has_no_combat_effect() {
    let mut game = Game::new();
    game.act(Action::Greet);
    assert!(game.message()[0].contains("No animal nearby"));
    let cases = [
        (CritterKind::Rabbit, "rabbit"),
        (CritterKind::Duck, "duck"),
        (CritterKind::Tortoise, "tortoise"),
    ];
    for (kind, expected) in cases {
        game.critters[0] = vec![Critter::new(17, 10, kind)];
        game.act(Action::Greet);
        assert!(game.message()[0].contains(expected));
        assert_eq!(game.hearts, 3);
        assert!(!game.sword);
        assert!(game.drain_sounds().next().is_none());
        game.critters[0][0].pos = Pos::new(18, 10);
        game.act(Action::Greet);
        assert!(game.message()[0].contains("No animal nearby"));
    }
    game.critters[0] = vec![
        Critter::new(17, 10, CritterKind::Rabbit),
        Critter::new(16, 10, CritterKind::Tortoise),
    ];
    game.act(Action::Greet);
    assert!(game.message()[0].contains("tortoise"));
}

#[test]
fn wildlife_and_visual_time_freeze_outside_play_and_restart_resets_them() {
    let mut game = Game::new();
    ticks(&mut game, 18);
    assert_ne!(game.wildlife(), Game::new().wildlife());
    for mode in [Mode::Paused, Mode::Dead, Mode::Won] {
        game.mode = mode;
        let wildlife = game.wildlife().to_vec();
        let time = game.animation_ms;
        let message = game.message();
        ticks(&mut game, 100);
        game.act(Action::Greet);
        assert_eq!(game.wildlife(), wildlife);
        assert_eq!(game.animation_ms, time);
        assert_eq!(game.message(), message);
    }
    game.act(Action::Restart);
    assert_eq!(game.wildlife(), Game::new().wildlife());
    assert_eq!(game.animation_ms, 0);
}

#[test]
fn inactive_rooms_keep_wildlife_state_and_screen_crossings_do_not_reset_it() {
    let mut game = Game::new();
    game.move_critters();
    let glade = game.wildlife().to_vec();
    exit(&mut game, Direction::North);
    ticks(&mut game, 100);
    assert_eq!(game.critters[Room::Glade.index()], glade);
    exit(&mut game, Direction::South);
    assert_eq!(game.wildlife(), glade);
    game.room = Room::Woods;
    game.move_critters();
    let woods = game.wildlife().to_vec();
    exit(&mut game, Direction::North);
    assert_eq!(game.room, Room::Woods);
    assert_eq!(game.wildlife(), woods);
}

#[test]
fn biome_branches_return_to_the_clearing_and_keep_the_woods_puzzle() {
    let mut game = Game::new();
    exit(&mut game, Direction::East);
    assert_eq!(game.room, Room::Clearing);
    for (out, room, back) in [
        (Direction::North, Room::Snow, Direction::South),
        (Direction::South, Room::Desert, Direction::North),
    ] {
        exit(&mut game, out);
        assert_eq!(game.room, room);
        assert_eq!(game.player, back.entrance());
        exit(&mut game, back);
        assert_eq!(game.room, Room::Clearing);
        assert_eq!(game.player, out.entrance());
    }
    exit(&mut game, Direction::East);
    for direction in ROUTE {
        exit(&mut game, direction);
    }
    assert_eq!(game.room, Room::Secret);
}

#[test]
fn equipment_collects_once_survives_travel_and_resets_on_restart() {
    let mut game = Game::new();
    game.room = Room::Desert;
    game.player = ARMOR;
    game.tick();
    assert!(game.armor);
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Pickup]);
    game.tick();
    assert!(game.drain_sounds().next().is_none());
    exit(&mut game, Direction::North);
    exit(&mut game, Direction::North);
    game.player = FROST_SWORD;
    game.tick();
    assert!(game.frost_sword && game.sword && game.armor);
    assert_eq!(game.sword_reach(), 2);
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Pickup]);
    game.tick();
    assert!(game.drain_sounds().next().is_none());
    exit(&mut game, Direction::South);
    exit(&mut game, Direction::West);
    exit(&mut game, Direction::North);
    game.player = SWORD;
    game.tick();
    assert!(
        game.frost_sword,
        "basic sword pickup must never downgrade frost sword"
    );
    assert!(game.drain_sounds().next().is_none());
    game.mode = Mode::Dead;
    game.act(Action::Restart);
    assert!(!game.armor && !game.frost_sword && !game.sword);
    assert!(!game.half_heart);
}

#[test]
fn armor_halves_damage_preserves_immunity_and_allows_six_contacts() {
    let mut game = Game::new();
    game.armor = true;
    game.enemies[0] = vec![Enemy::new(15, 10, Kind::Slime)];
    for hit in 1..=6 {
        game.player = START;
        game.immune_ms = 0;
        game.contact();
        assert_eq!(game.hearts * 2 + u8::from(game.half_heart), 6 - hit);
        assert_eq!(game.mode, if hit == 6 { Mode::Dead } else { Mode::Playing });
        game.player = START;
        game.contact();
        assert_eq!(game.hearts * 2 + u8::from(game.half_heart), 6 - hit);
    }
}

#[test]
fn tougher_enemies_take_two_swings_not_two_ticks_and_frost_hits_once() {
    for frost in [false, true] {
        let mut game = Game::new();
        game.sword = true;
        game.frost_sword = frost;
        game.facing = Direction::East;
        game.enemies[0] = vec![Enemy::tough(16, 10, Kind::Beetle)];
        game.act(Action::Attack);
        assert_eq!(game.actors()[0].health, if frost { 0 } else { 1 });
        assert_eq!(game.actors()[0].alive, !frost);
        ticks(&mut game, 3);
        assert_eq!(game.actors()[0].health, if frost { 0 } else { 1 });
        ticks(&mut game, 3);
        game.act(Action::Attack);
        assert!(!game.actors()[0].alive);
    }
}

#[test]
fn frost_reaches_two_tiles_in_each_direction_without_side_or_rear_hits() {
    for direction in [
        Direction::North,
        Direction::West,
        Direction::South,
        Direction::East,
    ] {
        let mut game = Game::new();
        game.sword = true;
        game.frost_sword = true;
        game.player = Pos::new(15, 8);
        game.facing = direction;
        let one = game.player.step(direction);
        let two = one.step(direction);
        let three = two.step(direction);
        let rear = game.player.step(direction.opposite());
        let side = game.player.step(
            if matches!(direction, Direction::North | Direction::South) {
                Direction::East
            } else {
                Direction::North
            },
        );
        game.enemies[0] = [one, two, three, rear, side]
            .into_iter()
            .map(|p| Enemy::tough(p.x, p.y, Kind::Slime))
            .collect();
        game.act(Action::Attack);
        assert!(!game.actors()[0].alive && !game.actors()[1].alive);
        assert!(game.actors()[2..].iter().all(|e| e.alive));
    }
}

#[test]
fn upgraded_blade_stops_at_water_and_walls_and_leaves_wildlife_safe() {
    let mut game = Game::new();
    game.room = Room::Clearing;
    game.player = Pos::new(20, 8);
    game.facing = Direction::South;
    game.sword = true;
    game.frost_sword = true;
    game.enemies[2] = vec![
        Enemy::tough(20, 9, Kind::Slime),
        Enemy::tough(20, 10, Kind::Slime),
    ];
    game.act(Action::Attack);
    assert!(!game.actors()[0].alive);
    assert!(game.actors()[1].alive);
    assert_eq!(game.attack_tiles().collect::<Vec<_>>(), [Pos::new(20, 9)]);
    let wildlife = game.wildlife().to_vec();
    game.player = Pos::new(6, 2);
    game.facing = Direction::South;
    ticks(&mut game, 6);
    game.act(Action::Attack);
    assert!(game.attack_tiles().next().is_none());
    assert_eq!(game.wildlife(), wildlife);
}

#[test]
fn containers_drop_only_on_death_and_never_duplicate() {
    let mut game = Game::new();
    game.sword = true;
    game.facing = Direction::East;
    let pos = START.step(Direction::East);
    game.enemies[0] = vec![Enemy::tough(pos.x, pos.y, Kind::Slime).with_container()];
    game.act(Action::Attack);
    assert!(game.actors()[0].alive);
    assert!(game.containers().is_empty());
    ticks(&mut game, 6);
    game.act(Action::Attack);
    assert!(!game.actors()[0].alive);
    assert_eq!(game.containers(), [pos]);
    ticks(&mut game, 6);
    game.act(Action::Attack);
    assert_eq!(game.containers(), [pos]);
    game.enemies[0].push(Enemy::new(pos.x, pos.y, Kind::Beetle));
    ticks(&mut game, 6);
    game.act(Action::Attack);
    assert_eq!(
        game.containers(),
        [pos],
        "ordinary enemies must not also drop containers"
    );
}

#[test]
fn walking_onto_a_container_increases_capacity_refills_and_collects_once() {
    let mut game = Game::new();
    game.hearts = 1;
    game.half_heart = true;
    let pos = START.step(Direction::East);
    game.heart_containers[0].push(pos);
    game.act(Action::Move(Direction::East));
    assert_eq!(game.max_hearts, 4);
    assert_eq!(game.hearts, 4);
    assert!(!game.half_heart);
    assert!(game.containers().is_empty());
    assert_eq!(game.drain_sounds().collect::<Vec<_>>(), [SoundCue::Pickup]);
    game.tick();
    assert_eq!(game.max_hearts, 4);
    assert!(game.drain_sounds().next().is_none());
}

#[test]
fn dropped_containers_and_capacity_persist_across_rooms_until_restart() {
    let mut game = Game::new();
    game.room = Room::Clearing;
    game.sword = true;
    game.player = Pos::new(9, 8);
    game.facing = Direction::East;
    game.act(Action::Attack);
    let drop = game.containers()[0];
    exit(&mut game, Direction::West);
    ticks(&mut game, 6);
    exit(&mut game, Direction::East);
    assert_eq!(game.containers(), [drop]);
    assert!(!game.actors()[0].alive);
    game.player = drop;
    game.collect();
    exit(&mut game, Direction::North);
    assert_eq!(game.max_hearts, 4);
    game.mode = Mode::Dead;
    game.act(Action::Restart);
    assert_eq!(game.max_hearts, 3);
    assert_eq!(game.hearts, 3);
    assert!(game.heart_containers.iter().all(Vec::is_empty));
    assert!(game.enemies.iter().flatten().all(|e| e.alive));
}

#[test]
fn maximum_health_is_bounded_at_six_even_with_extra_containers() {
    let mut game = Game::new();
    for expected in [4, 5, 6, 6] {
        game.heart_containers[0].push(START);
        game.collect();
        assert_eq!(game.max_hearts, expected);
        assert_eq!(game.hearts, expected);
        assert!(game.containers().is_empty());
    }
    let droppers = Game::new()
        .enemies
        .into_iter()
        .flatten()
        .filter(|e| e.drops_container)
        .count();
    assert_eq!(
        droppers, 3,
        "three designated enemies allow three capacity upgrades"
    );
}

#[test]
fn regeneration_restores_a_half_heart_every_twenty_seconds_and_stops_at_capacity() {
    let mut game = Game::new();
    game.hearts = 2;
    ticks(&mut game, usize::from(REGEN_MS / TICK_MS) - 1);
    assert_eq!(game.health_halves(), 4);
    game.tick();
    assert_eq!(game.health_halves(), 5);
    ticks(&mut game, usize::from(REGEN_MS / TICK_MS));
    assert_eq!(game.health_halves(), 6);
    ticks(&mut game, 1000);
    assert_eq!(game.health_halves(), 6);
    assert_eq!(game.regen_ms, 0);
    assert!(
        game.drain_sounds().next().is_none(),
        "passive regeneration should stay quiet"
    );
}

#[test]
fn damage_restarts_regeneration_and_nonplaying_modes_freeze_it() {
    let mut game = Game::new();
    game.hearts = 2;
    ticks(&mut game, 399);
    game.enemies[0].push(Enemy::new(START.x, START.y, Kind::Slime));
    game.contact();
    assert_eq!(game.health_halves(), 2);
    assert_eq!(game.regen_ms, 0);
    game.enemies[0].clear();
    ticks(&mut game, 399);
    assert_eq!(game.health_halves(), 2);
    for mode in [Mode::Paused, Mode::Dead, Mode::Won] {
        game.mode = mode;
        let timer = game.regen_ms;
        ticks(&mut game, 1000);
        assert_eq!(game.regen_ms, timer);
        assert_eq!(game.health_halves(), 2);
    }
    game.mode = Mode::Playing;
    game.tick();
    assert_eq!(game.health_halves(), 3);
}
