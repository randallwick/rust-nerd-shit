//! Handcrafted rooms. Coordinates are tiles, never terminal columns.

pub const WIDTH: i16 = 30;
pub const HEIGHT: i16 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    pub x: i16,
    pub y: i16,
}

impl Pos {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    pub fn step(self, direction: Direction) -> Self {
        let (x, y) = match direction {
            Direction::North => (0, -1),
            Direction::West => (-1, 0),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
        };
        Self::new(self.x + x, self.y + y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    North,
    West,
    South,
    East,
}

impl Direction {
    pub fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::West => Self::East,
            Self::South => Self::North,
            Self::East => Self::West,
        }
    }

    pub fn doorway(self) -> Pos {
        match self {
            Self::North => Pos::new(15, 0),
            Self::South => Pos::new(15, HEIGHT - 1),
            Self::West => Pos::new(0, 8),
            Self::East => Pos::new(WIDTH - 1, 8),
        }
    }

    pub fn entrance(self) -> Pos {
        self.doorway().step(self.opposite())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Room {
    Glade,
    Cave,
    Clearing,
    Woods,
    Secret,
}

impl Room {
    #[cfg(test)]
    pub const ALL: [Self; 5] = [
        Self::Glade,
        Self::Cave,
        Self::Clearing,
        Self::Woods,
        Self::Secret,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Glade => "Starting Glade",
            Self::Cave => "Sword Cave",
            Self::Clearing => "Unnecessary Clearing",
            Self::Woods => "Borrowed Woods",
            Self::Secret => "Secret Room",
        }
    }

    pub fn exits(self) -> &'static [Direction] {
        use Direction::*;
        match self {
            Self::Glade => &[North, East],
            Self::Cave => &[South],
            Self::Clearing => &[West, East],
            Self::Woods => &[North, West, South, East],
            Self::Secret => &[East],
        }
    }

    pub fn description(self) -> [&'static str; 2] {
        match self {
            Self::Glade => [
                "Sign: Sword cave north. Unnecessary trouble east.",
                "Find a sword, then discover what the woods are hiding.",
            ],
            Self::Cave => [
                "North to the leaves. West to the shade.",
                "South to your footprints. West to what's mislaid.",
            ],
            Self::Clearing => [
                "These monsters were hired for their enthusiasm.",
                "West: glade. East: woods. Space: performance review.",
            ],
            Self::Woods => [
                "The trees look suspiciously familiar.",
                "East leads back out. Remember the cave's inscription.",
            ],
            Self::Secret => [
                "The certificate has two extremely budget guards.",
                "Defeat them and claim it. East leads back to the woods.",
            ],
        }
    }
}

pub const SWORD: Pos = Pos::new(15, 5);
pub const HERMIT: Pos = Pos::new(15, 3);
pub const CHEST: Pos = Pos::new(15, 4);
pub const START: Pos = Pos::new(15, 10);

// Inclusive rectangles keep the small maps legible without a map-file parser.
struct Obstacle {
    left: i16,
    top: i16,
    right: i16,
    bottom: i16,
    glyph: char,
}

impl Obstacle {
    const fn new(left: i16, top: i16, right: i16, bottom: i16, glyph: char) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
            glyph,
        }
    }

    fn contains(&self, pos: Pos) -> bool {
        (self.left..=self.right).contains(&pos.x) && (self.top..=self.bottom).contains(&pos.y)
    }
}

fn obstacles(room: Room) -> &'static [Obstacle] {
    const GLADE: &[Obstacle] = &[
        Obstacle::new(3, 3, 7, 5, 'T'),
        Obstacle::new(21, 3, 25, 5, 'T'),
        Obstacle::new(4, 11, 9, 12, '~'),
        Obstacle::new(22, 11, 25, 12, 'T'),
    ];
    const CAVE: &[Obstacle] = &[
        Obstacle::new(5, 4, 8, 10, '#'),
        Obstacle::new(21, 4, 24, 10, '#'),
    ];
    const CLEARING: &[Obstacle] = &[
        Obstacle::new(6, 3, 9, 5, 'T'),
        Obstacle::new(20, 10, 23, 12, '~'),
        Obstacle::new(13, 11, 14, 12, 'T'),
    ];
    const WOODS: &[Obstacle] = &[
        Obstacle::new(4, 3, 10, 5, 'T'),
        Obstacle::new(19, 3, 25, 5, 'T'),
        Obstacle::new(4, 10, 10, 12, 'T'),
        Obstacle::new(19, 10, 25, 12, 'T'),
    ];
    const SECRET: &[Obstacle] = &[
        Obstacle::new(5, 3, 6, 5, '#'),
        Obstacle::new(23, 3, 24, 5, '#'),
        Obstacle::new(5, 10, 6, 12, '#'),
        Obstacle::new(23, 10, 24, 12, '#'),
    ];
    match room {
        Room::Glade => GLADE,
        Room::Cave => CAVE,
        Room::Clearing => CLEARING,
        Room::Woods => WOODS,
        Room::Secret => SECRET,
    }
}

pub fn inside(pos: Pos) -> bool {
    (0..WIDTH).contains(&pos.x) && (0..HEIGHT).contains(&pos.y)
}

pub fn tile(room: Room, pos: Pos) -> char {
    if !inside(pos) {
        return '#';
    }
    if pos.x == 0 || pos.y == 0 || pos.x == WIDTH - 1 || pos.y == HEIGHT - 1 {
        return if room.exits().iter().any(|d| d.doorway() == pos) {
            '.'
        } else {
            '#'
        };
    }
    if room == Room::Cave && pos == HERMIT {
        return 'H';
    }
    obstacles(room)
        .iter()
        .find(|o| o.contains(pos))
        .map_or('.', |o| o.glyph)
}

pub fn walkable(room: Room, pos: Pos) -> bool {
    tile(room, pos) == '.'
}
