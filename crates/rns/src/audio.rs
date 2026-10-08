//! Embedded audio; playback never blocks the game waiting for a sound to end.

use std::io::Cursor;
use std::sync::mpsc::{self, Receiver};

use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

use crate::game::{Mode, SoundCue};

const THEME: &[u8] = include_bytes!("../assets/audio/quest.wav");
const EFFECTS: [&[u8]; 7] = [
    include_bytes!("../assets/audio/sword.wav"),
    include_bytes!("../assets/audio/hit.wav"),
    include_bytes!("../assets/audio/hurt.wav"),
    include_bytes!("../assets/audio/pickup.wav"),
    include_bytes!("../assets/audio/secret.wav"),
    include_bytes!("../assets/audio/death.wav"),
    include_bytes!("../assets/audio/victory.wav"),
];
const MUSIC_VOLUME: f32 = 0.28;
const EFFECT_VOLUME: f32 = 0.25;
// Combined worst-case peaks remain below full scale even with four effects.
const MAX_EFFECTS: usize = 4;

fn decode(bytes: &'static [u8]) -> Result<SamplesBuffer, String> {
    let decoder = Decoder::try_from(Cursor::new(bytes))
        .map_err(|error| format!("cannot decode embedded audio: {error}"))?;
    Ok(SamplesBuffer::new(
        decoder.channels(),
        decoder.sample_rate(),
        decoder.collect::<Vec<_>>(),
    ))
}

struct SoundBank {
    theme: SamplesBuffer,
    effects: Vec<SamplesBuffer>,
}

impl SoundBank {
    fn load() -> Result<Self, String> {
        Ok(Self {
            theme: decode(THEME)?,
            effects: EFFECTS.into_iter().map(decode).collect::<Result<_, _>>()?,
        })
    }

    fn effect(&self, cue: SoundCue) -> Option<SamplesBuffer> {
        let index = match cue {
            SoundCue::Sword => 0,
            SoundCue::Hit => 1,
            SoundCue::Hurt => 2,
            SoundCue::Pickup => 3,
            SoundCue::Secret => 4,
            SoundCue::Death => 5,
            SoundCue::Victory => 6,
            SoundCue::Restart => return None,
        };
        Some(self.effects[index].clone())
    }
}

struct Playback {
    music: Sink,
    effects: Vec<Sink>,
    bank: SoundBank,
}

impl Playback {
    fn new(mixer: &Mixer, bank: SoundBank) -> Self {
        let music = Self::music(mixer, &bank);
        Self {
            music,
            effects: Vec::new(),
            bank,
        }
    }

    fn music(mixer: &Mixer, bank: &SoundBank) -> Sink {
        let music = Sink::connect_new(mixer);
        // Configure before appending: no brief full-volume blast on creation.
        music.pause();
        music.set_volume(MUSIC_VOLUME);
        music.append(bank.theme.clone().repeat_infinite());
        music
    }

    fn update(
        &mut self,
        mixer: &Mixer,
        cues: impl Iterator<Item = SoundCue>,
        mode: Mode,
        fits: bool,
        muted: bool,
    ) {
        self.effects.retain(|sink| !sink.empty());
        for cue in cues {
            if cue == SoundCue::Restart {
                self.effects.clear();
                // Drop the old sink instead of Sink::clear(), which waits for
                // the audio callback and could hang if a device disconnects.
                self.music = Self::music(mixer, &self.bank);
            }
            if matches!(cue, SoundCue::Death | SoundCue::Victory) {
                self.effects.clear();
            }
            if let Some(source) = self.bank.effect(cue).filter(|_| !muted) {
                if self.effects.len() == MAX_EFFECTS {
                    self.effects.remove(0);
                }
                let sink = Sink::connect_new(mixer);
                sink.pause();
                sink.set_volume(EFFECT_VOLUME);
                sink.append(source);
                self.effects.push(sink);
            }
        }
        self.music
            .set_volume(if muted { 0.0 } else { MUSIC_VOLUME });
        if fits && mode == Mode::Playing {
            self.music.play();
        } else {
            self.music.pause();
        }
        if muted {
            self.effects.clear();
        }
        for sink in &self.effects {
            if fits && mode != Mode::Paused {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }
}

struct Device {
    playback: Playback,
    errors: Receiver<String>,
    // Keep the stream alive until its sinks have been dropped.
    stream: OutputStream,
}

impl Device {
    fn open() -> Result<Self, String> {
        let bank = SoundBank::load()?;
        let (sender, errors) = mpsc::sync_channel(1);
        let mut stream = OutputStreamBuilder::from_default_device()
            .map_err(|error| format!("cannot open the default audio device: {error}"))?
            .with_error_callback(move |error| {
                // The callback never writes into the terminal or waits for it.
                let _ = sender.try_send(format!("audio device stopped: {error}"));
            })
            .open_stream_or_fallback()
            .map_err(|error| format!("cannot start audio playback: {error}"))?;
        stream.log_on_drop(false);
        Ok(Self {
            playback: Playback::new(stream.mixer(), bank),
            errors,
            stream,
        })
    }
}

pub struct Audio {
    device: Option<Device>,
    muted: bool,
    warning: Option<String>,
}

impl Audio {
    pub fn new(muted: bool) -> Self {
        let mut audio = Self {
            device: None,
            muted,
            warning: None,
        };
        if !muted {
            audio.open();
        }
        audio
    }

    fn open(&mut self) {
        match Device::open() {
            Ok(device) => {
                self.device = Some(device);
                self.warning = None;
            }
            Err(error) => self.fail(error),
        }
    }

    fn fail(&mut self, message: String) {
        self.device = None;
        self.warning = Some(message);
    }

    pub fn toggle(&mut self) {
        if self.device.is_none() {
            self.muted = false;
            self.open();
        } else {
            self.muted = !self.muted;
        }
    }

    pub fn update(&mut self, cues: impl Iterator<Item = SoundCue>, mode: Mode, fits: bool) {
        if let Some(error) = self
            .device
            .as_ref()
            .and_then(|device| device.errors.try_recv().ok())
        {
            self.fail(error);
        }
        if let Some(device) = &mut self.device {
            device
                .playback
                .update(device.stream.mixer(), cues, mode, fits, self.muted);
        }
    }

    pub fn status(&self) -> &'static str {
        if self.muted {
            "muted"
        } else if self.device.is_some() {
            "on"
        } else {
            "unavailable"
        }
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_audio_decodes_without_clipping_or_silent_assets() {
        for bytes in std::iter::once(THEME).chain(EFFECTS) {
            let source = decode(bytes).unwrap();
            assert_eq!(source.channels(), 1);
            assert_eq!(source.sample_rate(), 22050);
            let samples: Vec<_> = source.collect();
            assert!(!samples.is_empty());
            let peak = samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
            assert!((0.60..0.80).contains(&peak));
            assert_eq!(samples.first(), Some(&0.0));
            assert_eq!(samples.last(), Some(&0.0));
        }
    }

    #[test]
    fn theme_repeats_at_exact_loop_boundary() {
        let theme = decode(THEME).unwrap();
        assert!((theme.total_duration().unwrap().as_secs_f64() - 26.6667).abs() < 0.001);
        let length = theme.clone().count();
        let beginning: Vec<_> = theme.clone().take(2048).collect();
        assert_eq!(
            theme
                .repeat_infinite()
                .skip(length)
                .take(2048)
                .collect::<Vec<_>>(),
            beginning
        );
    }

    #[test]
    fn playback_handles_pause_resize_mute_endings_and_restart_without_a_device() {
        let (mixer, _output) = rodio::mixer::mixer(1, 22050);
        let mut player = Playback::new(&mixer, SoundBank::load().unwrap());
        player.update(
            &mixer,
            [SoundCue::Sword].into_iter(),
            Mode::Playing,
            true,
            false,
        );
        assert!(!player.music.is_paused());
        assert_eq!(player.effects.len(), 1);
        assert!(!player.effects[0].is_paused());
        player.update(&mixer, [].into_iter(), Mode::Paused, true, false);
        assert!(player.music.is_paused());
        assert!(player.effects[0].is_paused());
        player.update(&mixer, [].into_iter(), Mode::Playing, false, false);
        assert!(player.music.is_paused());
        player.update(&mixer, [].into_iter(), Mode::Playing, true, true);
        assert_eq!(player.music.volume(), 0.0);
        assert!(player.effects.is_empty());
        for mode in [Mode::Dead, Mode::Won] {
            let cue = if mode == Mode::Dead {
                SoundCue::Death
            } else {
                SoundCue::Victory
            };
            player.update(&mixer, [cue].into_iter(), mode, true, false);
            assert!(player.music.is_paused());
            assert_eq!(player.effects.len(), 1);
            assert!(!player.effects[0].is_paused());
        }
        player.update(
            &mixer,
            [SoundCue::Restart].into_iter(),
            Mode::Playing,
            true,
            false,
        );
        assert!(!player.music.is_paused());
        assert_eq!(player.music.volume(), MUSIC_VOLUME);
        assert!(player.effects.is_empty());
    }

    #[test]
    fn effect_voices_are_bounded_even_when_input_is_batched() {
        let (mixer, _output) = rodio::mixer::mixer(1, 22050);
        let mut player = Playback::new(&mixer, SoundBank::load().unwrap());
        player.update(
            &mixer,
            [SoundCue::Sword; 32].into_iter(),
            Mode::Playing,
            true,
            false,
        );
        assert_eq!(player.effects.len(), MAX_EFFECTS);
        assert!(MUSIC_VOLUME * 0.65 + EFFECT_VOLUME * 0.75 * (MAX_EFFECTS as f32) < 1.0);
    }

    #[test]
    fn muted_start_skips_device_and_failure_remains_nonfatal() {
        let mut audio = Audio::new(true);
        assert!(audio.device.is_none());
        assert_eq!(audio.status(), "muted");
        assert!(audio.warning().is_none());
        audio.muted = false;
        audio.fail("test device disconnected".into());
        audio.update([SoundCue::Sword].into_iter(), Mode::Playing, true);
        assert_eq!(audio.status(), "unavailable");
        assert_eq!(audio.warning(), Some("test device disconnected"));
    }
}
