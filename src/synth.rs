use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;

use crate::note::Event;

/// ADSR envelope parameters (times in seconds, sustain as level 0.0..=1.0)
#[derive(Debug, Clone)]
pub struct Adsr {
    /// Time to rise from 0 to peak (seconds)
    pub attack: f64,
    /// Time to fall from peak to sustain level (seconds)
    pub decay: f64,
    /// Level held while key is down (0.0..=1.0)
    pub sustain: f64,
    /// Time to fall to zero after key release (seconds)
    pub release: f64,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.25,
        }
    }
}

/// Envelope stage for one voice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Compute current envelope level from voice state and ADSR params
fn envelope_level(
    stage: EnvStage,
    phase: f64,
    release_start: f64,
    adsr: &Adsr,
) -> f64 {
    match stage {
        EnvStage::Idle => 0.0,
        EnvStage::Attack => {
            if adsr.attack <= 0.0 {
                1.0
            } else {
                (phase / adsr.attack).min(1.0)
            }
        }
        EnvStage::Decay => {
            if adsr.decay <= 0.0 {
                adsr.sustain
            } else {
                let t = (phase / adsr.decay).min(1.0);
                1.0 + t * (adsr.sustain - 1.0)
            }
        }
        EnvStage::Sustain => adsr.sustain,
        EnvStage::Release => {
            if adsr.release <= 0.0 {
                0.0
            } else {
                let t = (phase / adsr.release).min(1.0);
                release_start * (1.0 - t)
            }
        }
    }
}

/// A command sent to the audio engine
#[derive(Clone, Debug)]
pub enum LiveCommand {
    /// Start playing a note on a track
    NoteOn {
        track: usize,
        key: char,
        freq: f64,
    },
    /// Stop a note on a track
    NoteOff { track: usize, key: char },
    /// Stop all notes (all tracks)
    AllNotesOff,
    /// Shut down the engine
    Shutdown,
}

/// A single playing voice with ADSR envelope
struct Voice {
    track: usize,
    key: char,
    freq: f64,
    phase: f64,
    env_stage: EnvStage,
    env_phase: f64,
    release_start_level: f64,
}

/// Peak amplitude of the oscillator (envelope scales this)
const PEAK_AMP: f64 = 0.3;

/// Audio engine that owns the cpal stream and accepts commands via a channel
pub struct AudioEngine {
    cmd_tx: mpsc::Sender<LiveCommand>,
    // Hold the stream to keep it alive; dropping it stops audio
    _stream: cpal::Stream,
}

impl AudioEngine {
    /// Create a new AudioEngine using the default audio output device and default ADSR (single track)
    pub fn new() -> Result<Self, String> {
        Self::with_adsr(Adsr::default())
    }

    /// Create a new AudioEngine with one custom ADSR (single track, track index 0)
    pub fn with_adsr(adsr: Adsr) -> Result<Self, String> {
        Self::with_instruments(vec![adsr])
    }

    /// Create a new AudioEngine with one ADSR per track (for song playback)
    pub fn with_instruments(adsrs: Vec<Adsr>) -> Result<Self, String> {
        if adsrs.is_empty() {
            return Err("at least one instrument required".to_string());
        }
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no output audio device available")?;

        let config = device
            .default_output_config()
            .map_err(|e| format!("failed to get default output config: {}", e))?;

        let sample_rate = config.sample_rate() as f64;
        let dt = 1.0 / sample_rate;

        let (cmd_tx, cmd_rx) = mpsc::channel::<LiveCommand>();

        let mut voices: Vec<Voice> = Vec::new();
        let adsrs = adsrs;

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        match cmd {
                            LiveCommand::NoteOn { track, key, freq } => {
                                if let Some(v) = voices
                                    .iter_mut()
                                    .find(|v| v.track == track && v.key == key)
                                {
                                    v.freq = freq;
                                    v.env_stage = EnvStage::Attack;
                                    v.env_phase = 0.0;
                                    v.release_start_level = 0.0;
                                } else {
                                    voices.push(Voice {
                                        track,
                                        key,
                                        freq,
                                        phase: 0.0,
                                        env_stage: EnvStage::Attack,
                                        env_phase: 0.0,
                                        release_start_level: 0.0,
                                    });
                                }
                            }
                            LiveCommand::NoteOff { track, key } => {
                                for v in voices.iter_mut() {
                                    if v.track == track
                                        && v.key == key
                                        && v.env_stage != EnvStage::Idle
                                    {
                                        let adsr = &adsrs[v.track];
                                        v.release_start_level = envelope_level(
                                            v.env_stage,
                                            v.env_phase,
                                            v.release_start_level,
                                            adsr,
                                        );
                                        v.env_stage = EnvStage::Release;
                                        v.env_phase = 0.0;
                                    }
                                }
                            }
                            LiveCommand::AllNotesOff => {
                                for v in voices.iter_mut() {
                                    if v.env_stage != EnvStage::Idle {
                                        let adsr = &adsrs[v.track];
                                        v.release_start_level = envelope_level(
                                            v.env_stage,
                                            v.env_phase,
                                            v.release_start_level,
                                            adsr,
                                        );
                                        v.env_stage = EnvStage::Release;
                                        v.env_phase = 0.0;
                                    }
                                }
                            }
                            LiveCommand::Shutdown => {
                                voices.clear();
                                for sample in data.iter_mut() {
                                    *sample = 0.0;
                                }
                                return;
                            }
                        }
                    }

                    for sample in data.iter_mut() {
                        let mut value = 0.0_f64;

                        for voice in voices.iter_mut() {
                            let adsr = &adsrs[voice.track];
                            match voice.env_stage {
                                EnvStage::Idle => {}
                                EnvStage::Attack => {
                                    voice.env_phase += dt;
                                    if voice.env_phase >= adsr.attack {
                                        voice.env_stage = EnvStage::Decay;
                                        voice.env_phase = 0.0;
                                    }
                                }
                                EnvStage::Decay => {
                                    voice.env_phase += dt;
                                    if voice.env_phase >= adsr.decay {
                                        voice.env_stage = EnvStage::Sustain;
                                        voice.env_phase = 0.0;
                                    }
                                }
                                EnvStage::Sustain => {}
                                EnvStage::Release => {
                                    voice.env_phase += dt;
                                    if voice.env_phase >= adsr.release {
                                        voice.env_stage = EnvStage::Idle;
                                    }
                                }
                            }

                            let level = envelope_level(
                                voice.env_stage,
                                voice.env_phase,
                                voice.release_start_level,
                                adsr,
                            );

                            if level > 0.0001 {
                                value += (voice.phase * 2.0 * std::f64::consts::PI).sin()
                                    * PEAK_AMP
                                    * level;
                                voice.phase += voice.freq / sample_rate;
                                if voice.phase >= 1.0 {
                                    voice.phase -= 1.0;
                                }
                            }
                        }

                        voices.retain(|v| v.env_stage != EnvStage::Idle);

                        *sample = value as f32;
                    }
                },
                move |err| {
                    eprintln!("audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("failed to build output stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("failed to play stream: {}", e))?;

        Ok(AudioEngine {
            cmd_tx,
            _stream: stream,
        })
    }

    /// Send a command to the audio thread
    pub fn send(&self, cmd: LiveCommand) -> Result<(), String> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| "audio thread disconnected".to_string())
    }
}

/// Play a single pattern through the given audio engine (track 0).
pub fn play_pattern_with_engine(
    pattern: &crate::note::Pattern,
    tempo: u32,
    engine: &AudioEngine,
) -> Result<(), String> {
    let beat_duration = 60.0 / tempo as f64;
    const TRACK: usize = 0;

    for event in &pattern.events {
        match event {
            Event::Note(n) => {
                let freq = n.note.to_freq(n.octave);
                println!("  Playing {:?}{} ({:.1} Hz)", n.note, n.octave, freq);
                engine.send(LiveCommand::NoteOn {
                    track: TRACK,
                    key: '\0',
                    freq,
                })?;
                std::thread::sleep(std::time::Duration::from_secs_f64(beat_duration));
                engine.send(LiveCommand::NoteOff {
                    track: TRACK,
                    key: '\0',
                })?;
            }
            Event::Chord(notes) => {
                let desc: Vec<String> = notes
                    .iter()
                    .map(|n| format!("{:?}{}", n.note, n.octave))
                    .collect();
                println!("  Playing chord [{}]", desc.join(" "));
                for (i, n) in notes.iter().enumerate() {
                    let freq = n.note.to_freq(n.octave);
                    let key = char::from(b'0' + i as u8);
                    engine.send(LiveCommand::NoteOn {
                        track: TRACK,
                        key,
                        freq,
                    })?;
                }
                std::thread::sleep(std::time::Duration::from_secs_f64(beat_duration));
                engine.send(LiveCommand::AllNotesOff)?;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Event::Rest(beats) => {
                let rest_duration = beat_duration * beats;
                println!("  Rest ({} beats)", beats);
                std::thread::sleep(std::time::Duration::from_secs_f64(rest_duration));
            }
            Event::BarLine => {}
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = engine.send(LiveCommand::Shutdown);

    Ok(())
}

/// Play a single pattern with default instrument (convenience for .notes file).
pub fn play_pattern(pattern: &crate::note::Pattern, tempo: u32) -> Result<(), String> {
    let engine = AudioEngine::new()?;
    play_pattern_with_engine(pattern, tempo, &engine)
}

/// Run a pre-sorted schedule of (beat, command); blocks until playback finishes.
pub fn play_schedule(
    schedule: &[crate::scheduler::ScheduledEvent],
    tempo: u32,
    engine: &AudioEngine,
) -> Result<(), String> {
    let beat_duration = 60.0 / tempo as f64;
    let start = std::time::Instant::now();

    for ev in schedule {
        let target_secs = ev.beat * beat_duration;
        let elapsed = start.elapsed().as_secs_f64();
        if target_secs > elapsed {
            std::thread::sleep(std::time::Duration::from_secs_f64(target_secs - elapsed));
        }
        engine.send(ev.command.clone())?;
    }

    // Let last notes ring out
    let last_beat = schedule.last().map(|e| e.beat).unwrap_or(0.0);
    std::thread::sleep(std::time::Duration::from_secs_f64(
        last_beat * beat_duration + 0.5 - start.elapsed().as_secs_f64(),
    ));
    let _ = engine.send(LiveCommand::Shutdown);
    Ok(())
}
