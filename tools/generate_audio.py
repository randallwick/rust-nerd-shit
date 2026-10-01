#!/usr/bin/env python3
"""Compose and render the game's original score and effects using only Python.

The MIDI and WAV share the same note events. No soundfonts, samples, network
access, or third-party packages are needed. Run with --check to check assets
without rewriting them.
"""

import argparse
import io
import math
from pathlib import Path
import random
import struct
import wave
from dataclasses import dataclass
from functools import lru_cache

ASSETS = Path(__file__).resolve().parents[1] / "crates/rns/assets/audio"
RATE = 22050
BPM = 144
PPQ = 480
BEATS = 64
SECONDS_PER_BEAT = 60 / BPM


@dataclass(frozen=True)
class Note:
    channel: int
    pitch: int
    beat: float
    duration: float
    velocity: int


def score():
    # "A Most Unnecessary Quest": 16 bars in A minor, with a brighter B phrase.
    # Each list is eight eighth notes. Zero is a rest. Entirely original music.
    melody = [
        [76, 0, 81, 79, 76, 72, 74, 76],
        [77, 0, 76, 72, 69, 72, 76, 77],
        [79, 76, 72, 0, 76, 79, 84, 83],
        [79, 74, 71, 74, 79, 0, 76, 74],
        [72, 76, 81, 0, 83, 81, 79, 76],
        [77, 74, 69, 74, 77, 81, 79, 77],
        [76, 77, 81, 77, 76, 72, 74, 0],
        [71, 76, 80, 83, 80, 76, 74, 71],
        [84, 0, 79, 76, 79, 84, 83, 79],
        [83, 81, 79, 74, 71, 74, 79, 0],
        [81, 77, 74, 0, 77, 81, 84, 81],
        [80, 76, 71, 76, 80, 83, 86, 83],
        [81, 76, 72, 76, 81, 83, 84, 81],
        [77, 81, 84, 81, 77, 76, 74, 72],
        [71, 74, 76, 80, 83, 80, 76, 71],
        [81, 0, 76, 72, 69, 0, 71, 74],
    ]
    chords = [
        (45, 57, 60, 64), (41, 57, 60, 65),
        (48, 55, 60, 64), (43, 55, 59, 62),
        (45, 57, 60, 64), (38, 57, 62, 65),
        (41, 57, 60, 65), (40, 56, 59, 64),
        (48, 55, 60, 64), (43, 55, 59, 62),
        (38, 57, 62, 65), (40, 56, 59, 64),
        (45, 57, 60, 64), (41, 57, 60, 65),
        (40, 56, 59, 64), (45, 57, 60, 64),
    ]
    notes = []
    for bar, (phrase, chord) in enumerate(zip(melody, chords)):
        for eighth, pitch in enumerate(phrase):
            if pitch:
                notes.append(Note(0, pitch, bar * 4 + eighth / 2, 0.43, 88))
        for eighth, degree in enumerate([1, 2, 3, 2, 1, 3, 2, 3]):
            notes.append(Note(1, chord[degree], bar * 4 + eighth / 2, 0.34, 48))
        for beat in range(4):
            notes.append(Note(2, chord[0] + (7 if beat % 2 else 0), bar * 4 + beat, 0.78, 86))
            notes.append(Note(9, 36 if beat % 2 == 0 else 38, bar * 4 + beat, 0.18, 64))
        for eighth in range(8):
            notes.append(Note(9, 42, bar * 4 + eighth / 2, 0.08, 30 if eighth % 2 else 42))
    return notes


def vlq(value):
    """MIDI variable-length quantity, most significant group first."""
    result = [value & 0x7F]
    while value >> 7:
        value >>= 7
        result.insert(0, (value & 0x7F) | 0x80)
    return bytes(result)


def chunk(kind, data):
    return kind + struct.pack(">I", len(data)) + data


def track(events):
    data = bytearray()
    previous = 0
    # Event priority places note-offs before new notes at the same tick.
    for tick, priority, event in sorted(events, key=lambda e: (e[0], e[1])):
        data.extend(vlq(tick - previous))
        data.extend(event)
        previous = tick
    data.extend(vlq(BEATS * PPQ - previous) + b"\xff\x2f\x00")
    return chunk(b"MTrk", data)


def midi(notes):
    title = b"A Most Unnecessary Quest"
    tempo = round(60_000_000 / BPM).to_bytes(3, "big")
    tracks = [track([
        (0, 0, b"\xff\x03" + vlq(len(title)) + title),
        (0, 0, b"\xff\x51\x03" + tempo),
        (0, 0, b"\xff\x58\x04\x04\x02\x18\x08"),
        (0, 0, b"\xff\x59\x02\x00\x01"),
    ])]
    for channel, program, name in [(0, 80, b"Heroic pulse"), (1, 80, b"Overengineered arpeggio"), (2, 38, b"Triangle bass"), (9, 0, b"Tiny drums")]:
        events = [(0, 0, b"\xff\x03" + vlq(len(name)) + name), (0, 0, bytes([0xC0 | channel, program]))]
        for note in notes:
            if note.channel == channel:
                start = round(note.beat * PPQ)
                end = round((note.beat + note.duration) * PPQ)
                events.extend([
                    (start, 2, bytes([0x90 | channel, note.pitch, note.velocity])),
                    (end, 1, bytes([0x80 | channel, note.pitch, 0])),
                ])
        tracks.append(track(events))
    return chunk(b"MThd", struct.pack(">HHH", 1, len(tracks), PPQ)) + b"".join(tracks)


def frequency(pitch):
    return 440 * 2 ** ((pitch - 69) / 12)


@lru_cache(maxsize=None)
def oscillator(pitch, voice):
    # Band-limited wavetables keep the pulse timbre bright without harsh aliasing.
    limit = min(12, int(RATE / (2 * frequency(pitch))))
    table = []
    for i in range(2048):
        phase = 2 * math.pi * i / 2048
        if voice == 2:
            sample = sum((-1) ** ((n - 1) // 2) * math.sin(n * phase) / n ** 2 for n in range(1, limit + 1, 2)) * 8 / math.pi ** 2
        else:
            duty = 0.25 if voice == 0 else 0.125
            sample = sum(2 * math.sin(math.pi * n * duty) * math.cos(n * phase) / (math.pi * n) for n in range(1, limit + 1))
        table.append(sample)
    return table


def add_tone(samples, start, duration, pitch, amplitude, voice=0):
    first = round(start * RATE)
    length = min(round(duration * RATE), len(samples) - first)
    table = oscillator(pitch, voice)
    increment = frequency(pitch) * len(table) / RATE
    attack, release = RATE * 0.004, RATE * 0.035
    for i in range(max(0, length)):
        envelope = min(1, i / attack, (length - 1 - i) / release)
        samples[first + i] += table[int(i * increment) % len(table)] * envelope * amplitude


def add_drum(samples, start, pitch, amplitude, seed):
    first = round(start * RATE)
    length = round((0.16 if pitch == 36 else 0.09 if pitch == 38 else 0.035) * RATE)
    rng = random.Random(seed)
    phase = 0.0
    previous_noise = 0.0
    for i in range(min(length, len(samples) - first)):
        t = i / RATE
        envelope = min(1, i / 30) * (1 - i / length) ** 3
        noise = rng.uniform(-1, 1)
        if pitch == 36:
            phase += 2 * math.pi * (48 + 120 * math.exp(-t * 35)) / RATE
            sound = math.sin(phase)
        elif pitch == 38:
            sound = noise * 0.7 + math.sin(t * 2 * math.pi * 170) * 0.3
        else:
            sound = (noise - previous_noise) * 0.5
        samples[first + i] += sound * envelope * amplitude
        previous_noise = noise


def wav(samples, peak=0.75):
    maximum = max(abs(sample) for sample in samples) or 1
    gain = peak / maximum
    pcm = bytearray()
    for sample in samples:
        pcm.extend(struct.pack("<h", round(sample * gain * 32767)))
    output = io.BytesIO()
    with wave.open(output, "wb") as writer:
        writer.setparams((1, 2, RATE, len(samples), "NONE", "not compressed"))
        writer.writeframes(pcm)
    return output.getvalue()


def music(notes):
    samples = [0.0] * round(BEATS * SECONDS_PER_BEAT * RATE)
    for index, note in enumerate(notes):
        amplitude = note.velocity / 127
        if note.channel == 9:
            add_drum(samples, note.beat * SECONDS_PER_BEAT, note.pitch, amplitude * 0.18, index)
        else:
            add_tone(samples, note.beat * SECONDS_PER_BEAT, note.duration * SECONDS_PER_BEAT, note.pitch, amplitude * [0.30, 0.17, 0.26][note.channel], note.channel)
    return wav(samples, 0.65)


def effect(name):
    phrases = {
        "pickup": ([72, 76, 79, 84], 0.09, 0.55),
        "secret": ([69, 72, 76, 83, 81], 0.12, 0.85),
        "death": ([69, 68, 65, 62, 57, 45], 0.12, 1.05),
        "victory": ([69, 72, 76, 81, 0, 79, 81, 84, 88], 0.13, 1.55),
    }
    if name in phrases:
        pitches, interval, duration = phrases[name]
        samples = [0.0] * round(duration * RATE)
        for index, pitch in enumerate(pitches):
            if pitch:
                length = duration - index * interval if index == len(pitches) - 1 else interval * 0.9
                add_tone(samples, index * interval, length, pitch, 0.5)
                if name in ("secret", "victory"):
                    add_tone(samples, index * interval, length, pitch - 12, 0.25, 2)
        return wav(samples)
    duration = {"sword": 0.12, "hit": 0.10, "hurt": 0.28}[name]
    samples = [0.0] * round(duration * RATE)
    rng = random.Random(45)
    phase = 0.0
    for i in range(len(samples)):
        progress = i / (len(samples) - 1)
        envelope = min(1, i / (RATE * 0.003)) * (1 - progress) ** 2
        hz = {"sword": 1500 - 1100 * progress, "hit": 500 - 360 * progress, "hurt": 180 - 110 * progress}[name]
        phase += 2 * math.pi * hz / RATE
        pulse = math.tanh(math.sin(phase) * 3)
        noise = rng.uniform(-1, 1)
        sound = noise * 0.8 + pulse * 0.2 if name == "sword" else pulse * 0.8 + noise * 0.2
        samples[i] = sound * envelope
    return wav(samples)


def assets():
    notes = score()
    result = {"quest.mid": midi(notes), "quest.wav": music(notes)}
    result.update({f"{name}.wav": effect(name) for name in ["sword", "hit", "hurt", "pickup", "secret", "death", "victory"]})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify deterministic regeneration without writing")
    args = parser.parse_args()
    if not args.check:
        ASSETS.mkdir(parents=True, exist_ok=True)
    for name, data in assets().items():
        path = ASSETS / name
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                parser.exit(1, f"Audio asset is missing or out of date: {path}\n")
            print(f"Verified {name}")
        else:
            path.write_bytes(data)
            print(f"Generated {name} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
