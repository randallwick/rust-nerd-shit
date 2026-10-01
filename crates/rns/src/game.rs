//! Deterministic simulation. No terminal I/O, wall clock, files, or randomness.

use crate::world::{self, CHEST, Direction, Pos, Room, START, SWORD};

pub const TICK_MS: u16 = 50;
const MOVE_MS: u16 = 120;
const SLASH_MS: u16 = 150;
const ATTACK_MS: u16 = 300;
const IMMUNE_MS: u16 = 1000;
const ENEMY_MS: u16 = 600;
const ROUTE: [Direction; 4] = [
    Direction::North,
    Direction::West,
    Direction::South,
    Direction::West,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Playing,
    Paused,
    Dead,
    Won,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Move(Direction),
    Attack,
    Pause,
    Restart,
}

/// One-shot feedback emitted by successful game actions, never by rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundCue {
    Sword,
    Hit,
    Hurt,
    Pickup,
    Secret,
    Death,
    Victory,
    Restart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Slime,
    Beetle,
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub pos: Pos,
    pub kind: Kind,
    pub alive: bool,
    origin: Pos,
    phase: usize,
    direction: Direction,
}

impl Enemy {
    fn new(x: i16, y: i16, kind: Kind) -> Self {
        Self {
            pos: Pos::new(x, y),
            kind,
            alive: true,
            origin: Pos::new(x, y),
            phase: 0,
            direction: Direction::East,
        }
    }
}

pub struct Game {
    pub room: Room,
    pub player: Pos,
    pub facing: Direction,
    pub hearts: u8,
    pub sword: bool,
    pub mode: Mode,
    pub immune_ms: u16,
    pub slash_ms: u16,
    pub enemies: [Vec<Enemy>; 5],
    woods_progress: usize,
    move_ms: u16,
    attack_ms: u16,
    enemy_ms: u16,
    message_ms: u16,
    message: [&'static str; 2],
    sounds: Vec<SoundCue>,
}

impl Game {
    pub fn new() -> Self {
        use Kind::*;
        Self {
            room: Room::Glade,
            player: START,
            facing: Direction::North,
            hearts: 3,
            sword: false,
            mode: Mode::Playing,
            immune_ms: 0,
            slash_ms: 0,
            enemies: [
                vec![],
                vec![],
                vec![
                    Enemy::new(10, 8, Slime),
                    Enemy::new(18, 6, Beetle),
                    Enemy::new(24, 8, Slime),
                ],
                vec![Enemy::new(12, 6, Slime), Enemy::new(17, 9, Beetle)],
                vec![Enemy::new(12, 7, Slime), Enemy::new(18, 7, Beetle)],
            ],
            woods_progress: 0,
            move_ms: 0,
            attack_ms: 0,
            enemy_ms: ENEMY_MS,
            message_ms: 0,
            message: ["", ""],
            sounds: Vec::new(),
        }
    }

    pub fn actors(&self) -> &[Enemy] {
        &self.enemies[self.room.index()]
    }

    pub fn drain_sounds(&mut self) -> impl Iterator<Item = SoundCue> + '_ {
        self.sounds.drain(..)
    }

    pub fn message(&self) -> [&'static str; 2] {
        if self.message_ms > 0 {
            self.message
        } else {
            self.room.description()
        }
    }

    fn say(&mut self, message: [&'static str; 2]) {
        self.message = message;
        self.message_ms = 3500;
    }

    pub fn act(&mut self, action: Action) {
        if matches!(action, Action::Restart) && matches!(self.mode, Mode::Dead | Mode::Won) {
            *self = Self::new();
            self.sounds.push(SoundCue::Restart);
            return;
        }
        if matches!(action, Action::Pause) {
            self.mode = match self.mode {
                Mode::Playing => Mode::Paused,
                Mode::Paused => Mode::Playing,
                other => other,
            };
            return;
        }
        if self.mode != Mode::Playing {
            return;
        }
        match action {
            Action::Move(direction) => self.move_player(direction),
            Action::Attack => self.attack(),
            Action::Pause | Action::Restart => {}
        }
    }

    fn move_player(&mut self, direction: Direction) {
        if self.slash_ms > 0 || self.move_ms > 0 {
            return;
        }
        self.facing = direction;
        self.move_ms = MOVE_MS;
        let next = self.player.step(direction);
        if !world::inside(next) && self.player == direction.doorway() {
            self.cross(direction);
        } else if world::walkable(self.room, next) {
            self.player = next;
            self.contact();
            self.collect();
        }
    }

    fn attack(&mut self) {
        if !self.sword {
            self.say([
                "You need a sword. Visit the cave north of the glade.",
                "The hermit has one dependency worth taking.",
            ]);
            return;
        }
        if self.attack_ms > 0 {
            return;
        }
        self.attack_ms = ATTACK_MS;
        self.slash_ms = SLASH_MS;
        self.sounds.push(SoundCue::Sword);
        self.hit();
    }

    fn hit(&mut self) {
        let target = self.player.step(self.facing);
        for enemy in &mut self.enemies[self.room.index()] {
            if enemy.alive && enemy.pos == target {
                enemy.alive = false;
                self.sounds.push(SoundCue::Hit);
            }
        }
    }

    fn collect(&mut self) {
        if self.mode != Mode::Playing {
            return;
        }
        if self.room == Room::Cave && self.player == SWORD && !self.sword {
            self.sword = true;
            self.sounds.push(SoundCue::Pickup);
            self.say([
                "It's dangerous to go alone. Take this dependency.",
                "Sword equipped. Space strikes the tile you face.",
            ]);
        }
        if self.room == Room::Secret && self.player == CHEST {
            if self.actors().iter().all(|e| !e.alive) {
                self.mode = Mode::Won;
                self.sounds.push(SoundCue::Victory);
            } else {
                self.say([
                    "The certificate is guarded. Defeat both guards first.",
                    "Need a sword? Leave east and return to the glade's cave.",
                ]);
            }
        }
    }

    fn cross(&mut self, direction: Direction) {
        use Direction::*;
        let destination = match (self.room, direction) {
            (Room::Glade, North) => Some((Room::Cave, South)),
            (Room::Glade, East) => Some((Room::Clearing, West)),
            (Room::Cave, South) => Some((Room::Glade, North)),
            (Room::Clearing, West) => Some((Room::Glade, East)),
            (Room::Clearing, East) => {
                self.woods_progress = 0;
                Some((Room::Woods, West))
            }
            (Room::Woods, East) => {
                self.woods_progress = 0;
                Some((Room::Clearing, East))
            }
            (Room::Woods, direction) => {
                if direction == ROUTE[self.woods_progress] {
                    self.woods_progress += 1;
                } else {
                    self.woods_progress = 0;
                }
                if self.woods_progress == ROUTE.len() {
                    self.woods_progress = 0;
                    Some((Room::Secret, East))
                } else {
                    Some((Room::Woods, direction.opposite()))
                }
            }
            (Room::Secret, East) => {
                self.woods_progress = 0;
                Some((Room::Woods, West))
            }
            _ => None,
        };
        if let Some((room, entrance)) = destination {
            if room == Room::Secret {
                self.sounds.push(SoundCue::Secret);
            }
            self.room = room;
            self.player = entrance.entrance();
            self.immune_ms = IMMUNE_MS;
            self.enemy_ms = ENEMY_MS;
            self.message_ms = 0;
            // Relocate only an actor on the arrival tile; all others retain state.
            let index = room.index();
            for i in 0..self.enemies[index].len() {
                if !self.enemies[index][i].alive || self.enemies[index][i].pos != self.player {
                    continue;
                }
                let free = (1..world::HEIGHT - 1)
                    .flat_map(|y| (1..world::WIDTH - 1).map(move |x| Pos::new(x, y)))
                    .find(|p| {
                        *p != self.player
                            && world::walkable(room, *p)
                            && !self.actors().iter().any(|e| e.alive && e.pos == *p)
                    });
                if let Some(pos) = free {
                    self.enemies[index][i].pos = pos;
                    self.enemies[index][i].origin = pos;
                }
            }
        }
    }

    fn contact(&mut self) {
        if self.immune_ms > 0
            || !self
                .actors()
                .iter()
                .any(|e| e.alive && e.pos == self.player)
        {
            return;
        }
        self.hearts = self.hearts.saturating_sub(1);
        self.immune_ms = IMMUNE_MS;
        let retreat = self.player.step(self.facing.opposite());
        if world::walkable(self.room, retreat)
            && !self.actors().iter().any(|e| e.alive && e.pos == retreat)
        {
            self.player = retreat;
        }
        if self.hearts == 0 {
            self.mode = Mode::Dead;
            self.sounds.push(SoundCue::Death);
        } else {
            self.sounds.push(SoundCue::Hurt);
        }
    }

    pub fn tick(&mut self) {
        if self.mode != Mode::Playing {
            return;
        }
        self.move_ms = self.move_ms.saturating_sub(TICK_MS);
        self.attack_ms = self.attack_ms.saturating_sub(TICK_MS);
        self.slash_ms = self.slash_ms.saturating_sub(TICK_MS);
        self.immune_ms = self.immune_ms.saturating_sub(TICK_MS);
        self.message_ms = self.message_ms.saturating_sub(TICK_MS);
        self.enemy_ms = self.enemy_ms.saturating_sub(TICK_MS);
        if self.enemy_ms == 0 {
            self.move_enemies();
            self.enemy_ms = ENEMY_MS;
        }
        if self.slash_ms > 0 {
            self.hit();
        }
        self.contact();
        self.collect();
    }

    fn move_enemies(&mut self) {
        const WANDER: [Direction; 8] = [
            Direction::East,
            Direction::South,
            Direction::West,
            Direction::West,
            Direction::North,
            Direction::East,
            Direction::North,
            Direction::South,
        ];
        let room = self.room;
        let index = room.index();
        for i in 0..self.enemies[index].len() {
            let enemy = &mut self.enemies[index][i];
            if !enemy.alive {
                continue;
            }
            let direction = match enemy.kind {
                Kind::Slime => {
                    let direction = WANDER[enemy.phase % WANDER.len()];
                    enemy.phase += 1;
                    direction
                }
                Kind::Beetle => enemy.direction,
            };
            let next = enemy.pos.step(direction);
            let patrol_ok = enemy.kind != Kind::Beetle || (next.x - enemy.origin.x).abs() <= 3;
            let can_move = patrol_ok
                && next.x > 0
                && next.x < world::WIDTH - 1
                && next.y > 0
                && next.y < world::HEIGHT - 1
                && world::walkable(room, next)
                && !self.enemies[index].iter().any(|e| e.alive && e.pos == next);
            if can_move {
                self.enemies[index][i].pos = next;
            } else if self.enemies[index][i].kind == Kind::Beetle {
                self.enemies[index][i].direction = direction.opposite();
            }
        }
    }
}

#[cfg(test)]
mod tests;
