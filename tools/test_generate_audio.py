"""Validate the actual MIDI artifact, including timing and balanced note events."""

from collections import Counter
import io
import struct
import unittest
import wave

import generate_audio as audio


def vlq(data, offset):
    value = 0
    for _ in range(4):
        byte = data[offset]
        offset += 1
        value = (value << 7) | (byte & 0x7F)
        if not byte & 0x80:
            return value, offset
    raise AssertionError("MIDI delta time exceeds four bytes")


class AudioArtifacts(unittest.TestCase):
    def test_midi_has_tempo_five_tracks_and_no_hanging_notes(self):
        data = (audio.ASSETS / "quest.mid").read_bytes()
        self.assertEqual(data[:4], b"MThd")
        self.assertEqual(struct.unpack(">IHHH", data[4:14]), (6, 1, 5, audio.PPQ))
        offset, tempo, note_count = 14, None, 0
        for track in range(5):
            self.assertEqual(data[offset:offset + 4], b"MTrk")
            length = struct.unpack(">I", data[offset + 4:offset + 8])[0]
            offset += 8
            end = offset + length
            active = Counter()
            tick, ended = 0, False
            while offset < end:
                delta, offset = vlq(data, offset)
                tick += delta
                status = data[offset]
                offset += 1
                if status == 0xFF:
                    kind = data[offset]
                    length, offset = vlq(data, offset + 1)
                    payload = data[offset:offset + length]
                    offset += length
                    if kind == 0x51:
                        self.assertEqual(track, 0)
                        self.assertEqual(length, 3)
                        tempo = int.from_bytes(payload, "big")
                    if kind == 0x2F:
                        self.assertEqual(length, 0)
                        self.assertEqual(tick, audio.BEATS * audio.PPQ)
                        self.assertEqual(offset, end)
                        ended = True
                elif status & 0xF0 == 0xC0:
                    self.assertLess(data[offset], 128)
                    offset += 1
                else:
                    self.assertIn(status & 0xF0, (0x80, 0x90))
                    pitch, velocity = data[offset:offset + 2]
                    self.assertLess(pitch, 128)
                    self.assertLess(velocity, 128)
                    offset += 2
                    note = (status & 0x0F, pitch)
                    if status & 0xF0 == 0x90 and velocity:
                        active[note] += 1
                        note_count += 1
                    else:
                        self.assertGreater(active[note], 0)
                        active[note] -= 1
            self.assertTrue(ended)
            self.assertTrue(all(value == 0 for value in active.values()))
        self.assertEqual(offset, len(data))
        self.assertEqual(note_count, len(audio.score()))
        self.assertAlmostEqual(60_000_000 / tempo, audio.BPM, places=3)

    def test_rendered_theme_matches_midi_duration_and_loop_seam(self):
        data = (audio.ASSETS / "quest.wav").read_bytes()
        with wave.open(io.BytesIO(data), "rb") as reader:
            self.assertEqual(reader.getnchannels(), 1)
            self.assertEqual(reader.getsampwidth(), 2)
            self.assertEqual(reader.getframerate(), audio.RATE)
            self.assertAlmostEqual(reader.getnframes() / reader.getframerate(), audio.BEATS * audio.SECONDS_PER_BEAT, places=4)
            samples = reader.readframes(reader.getnframes())
            self.assertEqual(samples[:2], b"\0\0")
            self.assertEqual(samples[-2:], b"\0\0")


if __name__ == "__main__":
    unittest.main()
