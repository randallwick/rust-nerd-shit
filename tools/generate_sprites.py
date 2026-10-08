#!/usr/bin/env python3
"""Original 16px pixel art. Standard-library-only, deterministic PNG atlas.

Each entry becomes a Sprite variant in src/sprites.rs. No downloaded artwork,
fonts, or runtime asset paths. Run --check to detect stale generated files.
"""
import argparse
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
SIZE = 16
COLS = 8
PALETTE = {
    '.': (0, 0, 0, 0),
    'k': (25, 34, 38, 255), 'K': (45, 56, 53, 255),
    'g': (64, 100, 65, 255), 'G': (87, 131, 72, 255),
    'l': (128, 164, 86, 255), 'L': (173, 196, 112, 255),
    't': (71, 57, 48, 255), 'T': (120, 87, 56, 255),
    's': (156, 135, 94, 255), 'S': (184, 164, 118, 255),
    'b': (39, 83, 102, 255), 'B': (54, 116, 133, 255),
    'c': (83, 154, 159, 255), 'C': (147, 198, 190, 255),
    'r': (145, 67, 67, 255), 'R': (210, 107, 88, 255),
    'p': (208, 144, 154, 255), 'P': (240, 189, 167, 255),
    'w': (222, 218, 189, 255), 'W': (250, 241, 208, 255),
    'y': (212, 163, 75, 255), 'Y': (249, 210, 111, 255),
    'v': (93, 78, 107, 255), 'V': (148, 120, 147, 255),
    'n': (76, 79, 78, 255), 'N': (113, 116, 106, 255),
    'i': (191, 212, 218, 255), 'I': (230, 241, 236, 255),
    'd': (191, 151, 88, 255), 'D': (220, 183, 118, 255),
}


class Tile:
    def __init__(self, fill='.'):
        self.pixels = [[fill] * SIZE for _ in range(SIZE)]

    def rect(self, x, y, w, h, color):
        for py in range(max(0, y), min(SIZE, y + h)):
            for px in range(max(0, x), min(SIZE, x + w)):
                self.pixels[py][px] = color
        return self

    def dots(self, points, color):
        for x, y in points:
            self.rect(x, y, 1, 1, color)
        return self

    def pattern(self, rows, x=0, y=0):
        for dy, row in enumerate(rows):
            for dx, color in enumerate(row):
                if color != '.':
                    self.rect(x + dx, y + dy, 1, 1, color)
        return self


def terrain(base, light, dark, variant=0):
    tile = Tile(base)
    for x, y in [(2, 3), (11, 1), (7, 10), (13, 13), (1, 12), (9, 5)]:
        tile.rect((x + variant * 3) % 16, y, 2, 1, light)
    tile.dots([(4, 8), (14, 6), (6, 14)], dark)
    return tile


def water(frame):
    tile = Tile('B')
    for x, y in [(1, 3), (8, 8), (2, 13)]:
        x = (x + frame * 3) % 12
        tile.rect(x, y, 4, 1, 'c').rect(x + 1, y + 1, 2, 1, 'b')
    return tile


def tree(variant):
    tile = Tile()
    tile.rect(6, 10, 5, 6, 'k').rect(7, 10, 3, 5, 'T').rect(7, 11, 1, 4, 's')
    tile.pattern([
        '.....kkkkkk.....',
        '...kkggggggkk...',
        '..kggGGGGGGggk..',
        '.kgGGllllGGGggk.',
        'kgGllllllGGGGggk',
        'kgGlllGGGGGGGggk',
        'kgGGGGGGGGGGgggk',
        '.kgGGGGGGGggggk.',
        'kgGGGGGGGGGggggk',
        'kgGGGGGGGggggggk',
        '.kggGGGgggggggk.',
        '..kkggggggggkk..',
        '....kkkkkkkk....',
    ])
    if variant:
        tile.dots([(4, 4), (5, 4), (3, 5), (10, 8)], 'L')
    return tile


def wall():
    tile = Tile('K')
    for x, y, w, h in [(0, 0, 7, 6), (8, 0, 8, 6), (0, 7, 3, 7), (4, 7, 9, 7), (14, 7, 2, 7)]:
        tile.rect(x, y, w, h, 'n').rect(x, y, w, 1, 'N')
    tile.dots([(2, 2), (11, 9), (6, 11)], 'K')
    return tile


def hero(direction, frame):
    tile = Tile()
    tile.pattern([
        '....kkkkkk......',
        '...kcccccck.....',
        '..kccccCCCCk....',
        '..kcccccccckk...',
        '...kTTTTTTk.....',
        '...kPPPPPPk.....',
        '...kPkPPkPk.....',
        '....kPPPPk......',
        '...kkcccckk.....',
        '..kPkcCCckPk....',
        '..kPkcCCckPk....',
        '...kkYYYYkk.....',
        '....kcccck......',
        '....kkkkkk......',
    ])
    if direction == 'North':
        tile.rect(4, 4, 6, 4, 'T').rect(4, 8, 6, 3, 'c')
    elif direction == 'East':
        tile.rect(4, 5, 3, 3, 'T').rect(10, 6, 1, 1, 'P').rect(9, 6, 1, 1, 'k')
    elif direction == 'West':
        tile.rect(7, 5, 3, 3, 'T').rect(3, 6, 1, 1, 'P').rect(4, 6, 1, 1, 'k')
    tile.rect(4 + frame, 14, 2, 2 - frame, 't')
    tile.rect(8 - frame, 14, 2, 1 + frame, 't')
    return tile


def rabbit(frame):
    tile = Tile()
    tile.pattern([
        '..kwk..kwk..',
        '..kWk..kWk..',
        '..kPk..kPk..',
        '..kWkkkkWk..',
        '.kWWWWWWWWk.',
        '.kWkWWWkWWk.',
        '.kWWWPWWWWk.',
        '..kwwWWwwk..',
        '.kwwwwwwwwk.',
        'kwwWWWwwwwwk',
        'kwwWWWWwwwwk',
        '.kkwwwwwwkk.',
        '..kwk..kwk..',
    ], 2, 1 - frame)
    return tile


def duck(frame):
    tile = Tile()
    tile.pattern([
        '.......kkkk...',
        '......kGGGGk..',
        '......kGkGGk..',
        '......kGGGGkY.',
        '......kwWWkYY.',
        '..kk..kwWk....',
        '.kwwkkwwwwk...',
        '.kwWWWWwwwwk..',
        '..kwWWwwwwwk..',
        '...kkkkkkkk...',
    ], 1, 3 + frame)
    tile.rect(3, 14, 9, 1, 'C')
    return tile


def tortoise(frame):
    tile = Tile()
    tile.pattern([
        '....kkkkkk.....',
        '...kgGGGGgk....',
        '..kgGlGlGGgk...',
        '..kGGgGgGGGk...',
        '.kgGgGlGgGGgkk.',
        '.kGGlGgGlGGgGk.',
        '.kgGgGlGgGGgGk.',
        '..kggGGGGggkk..',
        '...kkkkkkkk....',
    ], 0, 4)
    tile.rect(13, 9, 1, 1, 'k')
    for x in [3, 10]:
        tile.rect(x + frame, 13, 2, 2, 's')
    return tile


def slime(frame):
    tile = Tile()
    tile.pattern([
        '....kkkkkk....',
        '..kkRRRRRRkk..',
        '.kRRPPPRRRRRk.',
        'kRRPPPRRRRRRRk',
        'kRRRkRRRRkRRRk',
        'kRRRkRRRRkRRRk',
        '.kRRRRRRRRRRk.',
        '..kkrrrrrrkk..',
        '....kkkkkk....',
    ], 1, 5 - frame)
    return tile


def beetle(frame):
    tile = Tile()
    tile.pattern([
        '...k......k...',
        '....kkkkkk....',
        '..kkvVVVVvkk..',
        '.k.kVVVVVVk.k.',
        '...kVvkkvVk...',
        '.kkVVvkkvVVkk.',
        '...kVvkkvVk...',
        '.k.kvvkkvvk.k.',
        '..kkvvvvvvkk..',
        '....kkkkkk....',
        '...k......k...',
    ], 1, 3 + frame)
    return tile


def heart(full):
    tile = Tile().pattern([
        '..kkk.kkk..', '.kRRRkRRRk.', 'kRPRRRRRRRk', 'kRRRRRRRRRk',
        '.kRRRRRRRk.', '..kRRRRRk..', '...kRRRk...', '....kRk....', '.....k.....',
    ], 2, 3)
    if not full:
        tile.pixels = [['K' if c in 'RP' else c for c in row] for row in tile.pixels]
    return tile


def recolor(tile, replacements):
    tile.pixels = [[replacements.get(c, c) for c in row] for row in tile.pixels]
    return tile


def cactus():
    return Tile().pattern([
        '......kkk.......', '.....kGlGk......', '.....kGlGk......',
        '..kk.kGlGk.kk...', '.kGGkkGlGkkGGk..', '.kGGkkGlGkkGGk..',
        '.kGGGGGlGGGGGk..', '..kkkkGlGkkkk...', '.....kGlGk......',
        '.....kGlGk......', '.....kGlGk......', '.....kGlGk......',
        '.....kGGGk......', '......kkk.......',
    ], 0, 2)


def snow_pine():
    return Tile().rect(7, 12, 3, 4, 'T').pattern([
        '.......II.......', '......IIII......', '.....IIiiII.....',
        '....IIggggII....', '.....kkkkkk.....', '....IIIIIIII....',
        '...IIiiiiiIII...', '..IIggggggggII..', '...kkkkkkkkkk...',
        '..IIIIIIIIIIII..', '.IIiiiiiiiiiiII.', 'IIggggggggggggII',
        '.kkkkkkkkkkkkkk.',
    ])


def armor():
    return Tile().pattern([
        '....kkkkkkkk....', '..kkNNNNNNNNkk..', '.kNIIkNNNNkIINk.',
        '.kINNkNNNNkNNIk.', '.kkkkkNNNNkkkkk.', '....kNNNNNNk....',
        '....kNIIIIIk....', '....kNIYIIIk....', '....kNIIIIIk....',
        '....kNNNNNNk....', '....kkkkkkkk....',
    ], 0, 2)


def blade(frost=False):
    tile = Tile().pattern([
        '.......W........', '......kWW.......', '......kWW.......',
        '......kWW.......', '......kWW.......', '......kWW.......',
        '......kWW.......', '......kWW.......', '......kWW.......',
        '......kWW.......', '.....YYYYYY.....', '.......T........',
        '.......T........', '.......Y........',
    ])
    return recolor(tile, {'W': 'I', 'Y': 'C'}) if frost else tile


def half_heart():
    tile = heart(True)
    for row in tile.pixels:
        for x, color in enumerate(row):
            if x >= 8 and color in 'RP':
                row[x] = 'K'
    return tile


def sprites():
    entries = [
        ('Grass', terrain('G', 'l', 'g')),
        ('GrassAlt', terrain('G', 'l', 'g', 1)),
        ('WoodsFloor', terrain('g', 'G', 'K')),
        ('Path', terrain('S', 's', 's')),
        ('CaveFloor', terrain('t', 'T', 'K')),
        ('StoneFloor', terrain('n', 'N', 'K')),
        ('Water', water(0)), ('WaterAlt', water(1)),
        ('Tree', tree(0)), ('TreeAlt', tree(1)), ('Wall', wall()),
        ('Flowers', Tile().dots([(4, 5), (10, 11)], 'G').rect(3, 4, 3, 1, 'p').rect(4, 3, 1, 3, 'p').rect(4, 4, 1, 1, 'Y').rect(9, 10, 3, 1, 'W').rect(10, 9, 1, 3, 'W').rect(10, 10, 1, 1, 'y')),
        ('Reeds', Tile().rect(3, 7, 1, 7, 'l').rect(8, 5, 1, 9, 'l').rect(12, 9, 1, 5, 'G').rect(7, 3, 3, 3, 'T').rect(2, 5, 3, 3, 'T')),
        ('Sword', Tile().pattern(['.......Wk.......', '......WWk.......', '.....WWk........', '....WWk.........', '...WWk..........', '..WWk...........', '.kYk............', '..kYk...........', '...kTk..........', '....kTk.........'], 2, 2)),
        ('Chest', Tile().pattern(['..kkkkkkkkkk..', '.kyYYYYYYYYyk.', 'kyYYYYYYYYYYyk', 'kyyyyyyyyyyyyk', 'kttttYYttttttk', 'kTTTTYYTTTTTTk', 'kTTTTTTTTTTTTk', '.kkkkkkkkkkkk.'], 1, 4)),
        ('Hermit', Tile().pattern(['....kkkkkk....', '...kvVVVVvk...', '..kvVVVVVVvk..', '..kkPPPPPPkk..', '...kPkPPkPk...', '...kWWWWWWk...', '....kWWWWk....', '...kvWWWWvk...', '..kvvvWWvvvk..', '..kvvvvvvvvk..', '..kvvvvvvvvk..', '...kkkkkkkk...'], 1, 2)),
    ]
    for direction in ['South', 'North', 'East', 'West']:
        for frame in range(2):
            entries.append((f'Hero{direction}{frame}', hero(direction, frame)))
    for name, make in [('Rabbit', rabbit), ('Duck', duck), ('Tortoise', tortoise), ('Slime', slime), ('Beetle', beetle)]:
        for frame in range(2):
            entries.append((f'{name}{frame}', make(frame)))
    entries.extend([('Heart', heart(True)), ('EmptyHeart', heart(False)),
        ('HalfHeart', half_heart()),
        ('Sand', terrain('D', 'd', 's')), ('SandAlt', terrain('D', 'd', 's', 1)),
        ('Cactus', cactus()),
        ('DesertRock', recolor(wall(), {'n': 'd', 'N': 'D', 'K': 's'})),
        ('Snow', terrain('I', 'w', 'i')), ('SnowAlt', terrain('I', 'w', 'i', 1)),
        ('SnowPine', snow_pine()),
        ('SnowRock', recolor(wall(), {'n': 'i', 'N': 'I', 'K': 'b'})),
        ('Ice', terrain('i', 'I', 'c')), ('Armor', armor()),
        ('Blade', blade()), ('FrostBlade', blade(True)),
        ('FrostSword', Tile().pattern([
            '.......Ik.......', '......IIk.......', '.....IIk........',
            '....IIk.........', '...IIk..........', '..IIk...........',
            '.kCk............', '..kCk...........', '...kTk..........', '....kTk.........',
        ], 2, 2)),
        ('HeartContainer', recolor(heart(True), {'k': 'Y'})),
    ])
    return entries


def png(entries):
    width, height = COLS * SIZE, ((len(entries) + COLS - 1) // COLS) * SIZE
    pixels = [[PALETTE['.']] * width for _ in range(height)]
    for index, (_, tile) in enumerate(entries):
        ox, oy = (index % COLS) * SIZE, (index // COLS) * SIZE
        for y, row in enumerate(tile.pixels):
            pixels[oy + y][ox:ox + SIZE] = [PALETTE[c] for c in row]
    raw = b''.join(b'\0' + bytes(c for pixel in row for c in pixel) for row in pixels)

    def chunk(kind, data):
        return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))

    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!2I5B', width, height, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='verify checked-in art without writing')
    args = parser.parse_args()
    entries = sprites()
    code = '// Generated by tools/generate_sprites.py. Edit the source art there.\n'
    code += '#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n#[repr(u16)]\npub enum Sprite {\n'
    code += ''.join(f'    {name},\n' for name, _ in entries) + '}\n'
    outputs = {
        ROOT / 'crates/rns/assets/sprites/atlas.png': png(entries),
        ROOT / 'crates/rns/src/sprites.rs': code.encode(),
    }
    for path, data in outputs.items():
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                parser.exit(1, f'Stale or missing sprite asset: {path}\n')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print(f'{"Verified" if args.check else "Generated"} {len(entries)} original sprites.')


if __name__ == '__main__':
    main()
