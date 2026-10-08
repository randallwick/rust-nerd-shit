//! Deterministic simulation. No terminal I/O, wall clock, files, or randomness.

use crate::world::{self, ARMOR, CHEST, Direction, FROST_SWORD, Pos, Room, START, SWORD};

pub const TICK_MS: u16 = 50;
const MOVE_MS: u16 = 120;
pub const SLASH_MS: u16 = 150;
const ATTACK_MS: u16 = 300;
const IMMUNE_MS: u16 = 1000;
const ENEMY_MS: u16 = 600;
const CRITTER_MS: u16 = 900;
const REGEN_MS: u16 = 20_000;
pub const MAX_HEARTS: u8 = 6;
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
    Greet,
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
    pub health: u8,
    hit_this_swing: bool,
    drops_container: bool,
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
            health: 1,
            hit_this_swing: false,
            drops_container: false,
            origin: Pos::new(x, y),
            phase: 0,
            direction: Direction::East,
        }
    }

    fn tough(x: i16, y: i16, kind: Kind) -> Self {
        Self {
            health: 2,
            ..Self::new(x, y, kind)
        }
    }

    fn with_container(mut self) -> Self {
        self.drops_container = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CritterKind {
    Rabbit,
    Duck,
    Tortoise,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Critter {
    pub pos: Pos,
    pub kind: CritterKind,
    origin: Pos,
    phase: usize,
}

impl Critter {
    fn new(x: i16, y: i16, kind: CritterKind) -> Self {
        Self {
            pos: Pos::new(x, y),
            kind,
            origin: Pos::new(x, y),
            phase: 0,
        }
    }

    fn habitat(&self, room: Room, pos: Pos) -> bool {
        let terrain = match self.kind {
            CritterKind::Duck => world::tile(room, pos) == '~',
            _ => world::walkable(room, pos),
        };
        terrain
            && pos.x > 1
            && pos.x < world::WIDTH - 2
            && pos.y > 1
            && pos.y < world::HEIGHT - 2
            && (pos.x - self.origin.x).abs() <= 3
            && (pos.y - self.origin.y).abs() <= 2
            && !matches!(
                (room, pos),
                (Room::Cave, SWORD)
                    | (Room::Secret, CHEST)
                    | (Room::Desert, ARMOR)
                    | (Room::Snow, FROST_SWORD)
            )
    }
}

pub struct Game {
    pub room: Room,
    pub player: Pos,
    pub facing: Direction,
    pub hearts: u8,
    pub max_hearts: u8,
    /// Whole hearts plus an optional half heart; armor removes half per hit.
    pub half_heart: bool,
    pub armor: bool,
    pub sword: bool,
    pub frost_sword: bool,
    pub mode: Mode,
    pub immune_ms: u16,
    pub slash_ms: u16,
    pub enemies: [Vec<Enemy>; Room::COUNT],
    pub critters: [Vec<Critter>; Room::COUNT],
    pub heart_containers: [Vec<Pos>; Room::COUNT],
    pub animation_ms: u16,
    pub impact: Option<Pos>,
    pub impact_ms: u16,
    woods_progress: usize,
    move_ms: u16,
    attack_ms: u16,
    enemy_ms: u16,
    critter_ms: u16,
    regen_ms: u16,
    message_ms: u16,
    message: [&'static str; 2],
    sounds: Vec<SoundCue>,
}

impl Game {
    pub fn new() -> Self {
        use CritterKind::*;
        use Kind::*;
        Self {
            room: Room::Glade,
            player: START,
            facing: Direction::North,
            hearts: 3,
            max_hearts: 3,
            half_heart: false,
            armor: false,
            sword: false,
            frost_sword: false,
            mode: Mode::Playing,
            immune_ms: 0,
            slash_ms: 0,
            enemies: [
                vec![],
                vec![],
                vec![
                    Enemy::new(10, 8, Slime).with_container(),
                    Enemy::new(18, 6, Beetle),
                    Enemy::new(24, 8, Slime),
                ],
                vec![Enemy::new(12, 6, Slime), Enemy::new(17, 9, Beetle)],
                vec![Enemy::new(12, 7, Slime), Enemy::new(18, 7, Beetle)],
                vec![
                    Enemy::tough(14, 6, Beetle).with_container(),
                    Enemy::tough(17, 11, Beetle),
                ],
                vec![
                    Enemy::tough(12, 5, Slime).with_container(),
                    Enemy::tough(17, 7, Slime),
                ],
            ],
            critters: [
                vec![
                    Critter::new(11, 6, Rabbit),
                    Critter::new(6, 11, Duck),
                    Critter::new(19, 12, Tortoise),
                ],
                vec![],
                vec![Critter::new(4, 11, Rabbit), Critter::new(21, 11, Duck)],
                vec![Critter::new(14, 3, Rabbit), Critter::new(15, 12, Tortoise)],
                vec![],
                vec![Critter::new(6, 10, Duck), Critter::new(24, 8, Tortoise)],
                vec![Critter::new(25, 8, Rabbit)],
            ],
            animation_ms: 0,
            heart_containers: std::array::from_fn(|_| Vec::new()),
            impact: None,
            impact_ms: 0,
            woods_progress: 0,
            move_ms: 0,
            attack_ms: 0,
            enemy_ms: ENEMY_MS,
            critter_ms: CRITTER_MS,
            regen_ms: 0,
            message_ms: 0,
            message: ["", ""],
            sounds: Vec::new(),
        }
    }

    pub fn actors(&self) -> &[Enemy] {
        &self.enemies[self.room.index()]
    }

    pub fn wildlife(&self) -> &[Critter] {
        &self.critters[self.room.index()]
    }

    pub fn containers(&self) -> &[Pos] {
        &self.heart_containers[self.room.index()]
    }

    pub fn health_halves(&self) -> u8 {
        self.hearts * 2 + u8::from(self.half_heart)
    }

    pub fn walking(&self) -> bool {
        self.mode == Mode::Playing && self.move_ms > 0 && self.slash_ms == 0
    }

    pub fn sword_reach(&self) -> u8 {
        if self.frost_sword { 2 } else { 1 }
    }

    /// Tile reach stays narrow and directional even though the visual blade
    /// sweeps an arc. Solid terrain stops both basic and upgraded attacks.
    pub fn attack_tiles(&self) -> impl Iterator<Item = Pos> + '_ {
        let mut pos = self.player;
        (0..self.sword_reach())
            .map(move |_| {
                pos = pos.step(self.facing);
                pos
            })
            .take_while(|pos| world::walkable(self.room, *pos))
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
            Action::Greet => self.greet(),
            Action::Pause | Action::Restart => {}
        }
    }

    fn greet(&mut self) {
        let nearby = self
            .wildlife()
            .iter()
            .filter(|c| (c.pos.x - self.player.x).abs() + (c.pos.y - self.player.y).abs() <= 2)
            .min_by_key(|c| (c.pos.x - self.player.x).abs() + (c.pos.y - self.player.y).abs());
        let message = match nearby.map(|c| c.kind) {
            Some(CritterKind::Rabbit) => [
                "The rabbit accepts a gentle head scratch.",
                "It has no quests. An excellent work-life balance.",
            ],
            Some(CritterKind::Duck) => [
                "The duck offers a small, approving quack.",
                "Pond life is going swimmingly.",
            ],
            Some(CritterKind::Tortoise) => [
                "The tortoise nods at its own comfortable pace.",
                "Some adventures are best taken slowly.",
            ],
            None => [
                "No animal nearby. Try within two tiles of one.",
                "Rabbits, ducks and tortoises are peaceful company.",
            ],
        };
        self.say(message);
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
        for enemy in &mut self.enemies[self.room.index()] {
            enemy.hit_this_swing = false;
        }
        self.sounds.push(SoundCue::Sword);
        self.hit();
    }

    fn hit(&mut self) {
        let targets: Vec<_> = self.attack_tiles().collect();
        let damage = if self.frost_sword { 2 } else { 1 };
        for enemy in &mut self.enemies[self.room.index()] {
            if enemy.alive && !enemy.hit_this_swing && targets.contains(&enemy.pos) {
                enemy.hit_this_swing = true;
                enemy.health = enemy.health.saturating_sub(damage);
                enemy.alive = enemy.health > 0;
                if !enemy.alive && enemy.drops_container {
                    self.heart_containers[self.room.index()].push(enemy.pos);
                }
                self.impact = Some(enemy.pos);
                self.impact_ms = 200;
                self.sounds.push(SoundCue::Hit);
            }
        }
    }

    fn collect(&mut self) {
        if self.mode != Mode::Playing {
            return;
        }
        if let Some(index) = self.containers().iter().position(|pos| *pos == self.player) {
            self.heart_containers[self.room.index()].remove(index);
            self.max_hearts = (self.max_hearts + 1).min(MAX_HEARTS);
            self.hearts = self.max_hearts;
            self.half_heart = false;
            self.regen_ms = 0;
            self.sounds.push(SoundCue::Pickup);
            self.say([
                "Heart container collected. Health fully restored.",
                "Maximum health grows by one heart, up to six.",
            ]);
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
        if self.room == Room::Desert && self.player == ARMOR && !self.armor {
            self.armor = true;
            self.sounds.push(SoundCue::Pickup);
            self.say([
                "Dune armor equipped. Contact damage is halved.",
                "A little less paperwork for your remaining hearts.",
            ]);
        }
        if self.room == Room::Snow && self.player == FROST_SWORD && !self.frost_sword {
            self.sword = true;
            self.frost_sword = true;
            self.sounds.push(SoundCue::Pickup);
            self.say([
                "Frost sword equipped: double damage, two-tile reach.",
                "Cold steel. Unreasonably warm confidence.",
            ]);
        }
    }

    fn cross(&mut self, direction: Direction) {
        use Direction::*;
        let destination = match (self.room, direction) {
            (Room::Glade, North) => Some((Room::Cave, South)),
            (Room::Glade, East) => Some((Room::Clearing, West)),
            (Room::Cave, South) => Some((Room::Glade, North)),
            (Room::Clearing, West) => Some((Room::Glade, East)),
            (Room::Clearing, North) => Some((Room::Snow, South)),
            (Room::Clearing, South) => Some((Room::Desert, North)),
            (Room::Snow, South) => Some((Room::Clearing, North)),
            (Room::Desert, North) => Some((Room::Clearing, South)),
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
        let half_hearts = self.health_halves();
        let remaining = half_hearts.saturating_sub(if self.armor { 1 } else { 2 });
        self.hearts = remaining / 2;
        self.half_heart = remaining % 2 == 1;
        self.regen_ms = 0;
        self.immune_ms = IMMUNE_MS;
        let retreat = self.player.step(self.facing.opposite());
        if world::walkable(self.room, retreat)
            && !self.actors().iter().any(|e| e.alive && e.pos == retreat)
        {
            self.player = retreat;
        }
        if self.hearts == 0 && !self.half_heart {
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
        self.animation_ms = (self.animation_ms + TICK_MS) % 2400;
        self.impact_ms = self.impact_ms.saturating_sub(TICK_MS);
        // Regenerate before contact: a damage event below restarts the timer.
        if self.health_halves() < self.max_hearts * 2 {
            self.regen_ms += TICK_MS;
            if self.regen_ms >= REGEN_MS {
                let health = self.health_halves() + 1;
                self.hearts = health / 2;
                self.half_heart = health % 2 == 1;
                self.regen_ms = 0;
            }
        } else {
            self.regen_ms = 0;
        }
        self.critter_ms = self.critter_ms.saturating_sub(TICK_MS);
        self.enemy_ms = self.enemy_ms.saturating_sub(TICK_MS);
        if self.enemy_ms == 0 {
            self.move_enemies();
            self.enemy_ms = ENEMY_MS;
        }
        if self.critter_ms == 0 {
            self.move_critters();
            self.critter_ms = CRITTER_MS;
        }
        if self.slash_ms > 0 {
            self.hit();
        }
        self.contact();
        self.collect();
    }

    fn move_critters(&mut self) {
        // Local, repeatable wandering with resting beats. Wildlife never blocks
        // the player or participates in combat; only the current room advances.
        const WANDER: [Option<Direction>; 8] = [
            Some(Direction::East),
            None,
            Some(Direction::South),
            Some(Direction::West),
            None,
            Some(Direction::West),
            Some(Direction::North),
            Some(Direction::East),
        ];
        let index = self.room.index();
        for i in 0..self.critters[index].len() {
            let critter = &mut self.critters[index][i];
            let phase = critter.phase;
            critter.phase = (phase + 1) % (WANDER.len() * 2);
            if critter.kind == CritterKind::Tortoise && phase % 2 == 1 {
                continue;
            }
            let beat = if critter.kind == CritterKind::Tortoise {
                phase / 2
            } else {
                phase % WANDER.len()
            };
            let Some(direction) = WANDER[beat] else {
                continue;
            };
            let next = critter.pos.step(direction);
            if critter.habitat(self.room, next)
                && next != self.player
                && !self.actors().iter().any(|e| e.alive && e.pos == next)
                && !self.wildlife().iter().any(|c| c.pos == next)
            {
                self.critters[index][i].pos = next;
            }
        }
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
#[path = "game/tests.rs"]
mod tests;
