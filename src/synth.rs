use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;

use crate::note::{Composition, Event};

/// A command sent to the audio engine
pub enum LiveCommand {
    /// Start playing a note triggered by a key
    NoteOn { key: char, freq: f64 },
    /// Stop a note triggered by a key
    NoteOff { key: char },
    /// Stop all notes
    AllNotesOff,
    /// Shut down the engine
    Shutdown,
}

/// A single playing voice
struct Voice {
    key: char,
    freq: f64,
    phase: f64,
    amplitude: f64,
    target_amp: f64,
    releasing: bool,
}

/// Audio engine that owns the cpal stream and accepts commands via a channel
pub struct AudioEngine {
    cmd_tx: mpsc::Sender<LiveCommand>,
    // Hold the stream to keep it alive; dropping it stops audio
    _stream: cpal::Stream,
}

/// Amplitude ramp time in seconds (prevents clicks)
const RAMP_TIME: f64 = 0.005;

impl AudioEngine {
    /// Create a new AudioEngine using the default audio output device
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no output audio device available")?;

        let config = device
            .default_output_config()
            .map_err(|e| format!("failed to get default output config: {}", e))?;

        let sample_rate = config.sample_rate() as f64;
        let ramp_increment = 1.0 / (RAMP_TIME * sample_rate);

        let (cmd_tx, cmd_rx) = mpsc::channel::<LiveCommand>();

        let mut voices: Vec<Voice> = Vec::new();

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Drain all pending commands
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        match cmd {
                            LiveCommand::NoteOn { key, freq } => {
                                // If this key already has a voice, retrigger it
                                if let Some(v) = voices.iter_mut().find(|v| v.key == key) {
                                    v.freq = freq;
                                    v.target_amp = 0.3;
                                    v.releasing = false;
                                } else {
                                    voices.push(Voice {
                                        key,
                                        freq,
                                        phase: 0.0,
                                        amplitude: 0.0,
                                        target_amp: 0.3,
                                        releasing: false,
                                    });
                                }
                            }
                            LiveCommand::NoteOff { key } => {
                                for v in voices.iter_mut() {
                                    if v.key == key {
                                        v.target_amp = 0.0;
                                        v.releasing = true;
                                    }
                                }
                            }
                            LiveCommand::AllNotesOff => {
                                for v in voices.iter_mut() {
                                    v.target_amp = 0.0;
                                    v.releasing = true;
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
                            // Ramp amplitude toward target
                            if voice.amplitude < voice.target_amp {
                                voice.amplitude =
                                    (voice.amplitude + ramp_increment).min(voice.target_amp);
                            } else if voice.amplitude > voice.target_amp {
                                voice.amplitude =
                                    (voice.amplitude - ramp_increment).max(voice.target_amp);
                            }

                            if voice.amplitude > 0.0001 {
                                value += (voice.phase * 2.0 * std::f64::consts::PI).sin()
                                    * voice.amplitude;
                                voice.phase += voice.freq / sample_rate;
                                if voice.phase >= 1.0 {
                                    voice.phase -= 1.0;
                                }
                            }
                        }

                        // Remove fully released voices
                        voices.retain(|v| !(v.releasing && v.amplitude <= 0.0001));

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

/// Play a composition through the default audio output (batch mode)
pub fn play(comp: &Composition) -> Result<(), String> {
    let engine = AudioEngine::new()?;

    let beat_duration = 60.0 / comp.tempo as f64;

    for track in &comp.tracks {
        for event in &track.events {
            match event {
                Event::Note(n) => {
                    let freq = n.note.to_freq(n.octave);
                    println!("  Playing {:?}{} ({:.1} Hz)", n.note, n.octave, freq);
                    engine.send(LiveCommand::NoteOn { key: '\0', freq })?;
                    std::thread::sleep(std::time::Duration::from_secs_f64(beat_duration));
                    engine.send(LiveCommand::NoteOff { key: '\0' })?;
                }
                Event::Chord(notes) => {
                    let desc: Vec<String> = notes
                        .iter()
                        .map(|n| format!("{:?}{}", n.note, n.octave))
                        .collect();
                    println!("  Playing chord [{}]", desc.join(" "));
                    for (i, n) in notes.iter().enumerate() {
                        let freq = n.note.to_freq(n.octave);
                        // Use index as a fake key to allow polyphonic chord voices
                        let key = char::from(b'0' + i as u8);
                        engine.send(LiveCommand::NoteOn { key, freq })?;
                    }
                    std::thread::sleep(std::time::Duration::from_secs_f64(beat_duration));
                    engine.send(LiveCommand::AllNotesOff)?;
                    // Brief ramp-down time
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Event::Rest(beats) => {
                    let rest_duration = beat_duration * beats;
                    println!("  Rest ({} beats)", beats);
                    std::thread::sleep(std::time::Duration::from_secs_f64(rest_duration));
                }
                Event::BarLine => {
                    // Bar lines are structural markers, no audio
                }
            }
        }
    }

    // Brief silence at the end so the last note rings out
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = engine.send(LiveCommand::Shutdown);

    Ok(())
}
