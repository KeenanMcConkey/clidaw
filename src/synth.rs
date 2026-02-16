use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;

use crate::note::{Composition, Event};

/// A command sent to the audio thread
enum AudioCommand {
    /// Play a note at a given frequency for a duration in seconds
    PlayNote { freq: f64, duration_secs: f64 },
    /// Play multiple frequencies simultaneously
    PlayChord { freqs: Vec<f64>, duration_secs: f64 },
    /// Silence for a duration
    Rest { duration_secs: f64 },
    /// Stop the stream
    Stop,
}

/// Play a composition through the default audio output
pub fn play(comp: &Composition) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no output audio device available")?;

    let config = device
        .default_output_config()
        .map_err(|e| format!("failed to get default output config: {}", e))?;

    let sample_rate = config.sample_rate() as f64;

    // Shared state for the audio callback
    let (cmd_tx, cmd_rx) = mpsc::channel::<AudioCommand>();

    // Audio generation state
    let mut phase: f64 = 0.0;
    let mut current_freqs: Vec<f64> = Vec::new();
    let mut samples_remaining: usize = 0;

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Check for new commands (non-blocking)
                if let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        AudioCommand::PlayNote { freq, duration_secs } => {
                            current_freqs = vec![freq];
                            samples_remaining = (duration_secs * sample_rate) as usize;
                            phase = 0.0;
                        }
                        AudioCommand::PlayChord { freqs, duration_secs } => {
                            current_freqs = freqs;
                            samples_remaining = (duration_secs * sample_rate) as usize;
                            phase = 0.0;
                        }
                        AudioCommand::Rest { duration_secs } => {
                            current_freqs.clear();
                            samples_remaining = (duration_secs * sample_rate) as usize;
                            phase = 0.0;
                        }
                        AudioCommand::Stop => {
                            current_freqs.clear();
                            samples_remaining = 0;
                            for sample in data.iter_mut() {
                                *sample = 0.0;
                            }
                            return;
                        }
                    }
                }

                for sample in data.iter_mut() {
                    if samples_remaining > 0 && !current_freqs.is_empty() {
                        let mut value = 0.0_f64;
                        for freq in &current_freqs {
                            value += (phase * freq * 2.0 * std::f64::consts::PI / sample_rate).sin();
                        }
                        // Normalize by number of voices and apply a gentle volume
                        value = value / current_freqs.len() as f64 * 0.3;
                        *sample = value as f32;
                        phase += 1.0;
                        samples_remaining -= 1;
                    } else if samples_remaining > 0 {
                        // Rest: output silence
                        *sample = 0.0;
                        samples_remaining -= 1;
                    } else {
                        *sample = 0.0;
                    }
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

    // Calculate beat duration from tempo
    let beat_duration = 60.0 / comp.tempo as f64;

    for track in &comp.tracks {
        for event in &track.events {
            match event {
                Event::Note(n) => {
                    let freq = n.note.to_freq(n.octave);
                    println!("  Playing {:?}{} ({:.1} Hz)", n.note, n.octave, freq);
                    cmd_tx
                        .send(AudioCommand::PlayNote {
                            freq,
                            duration_secs: beat_duration,
                        })
                        .map_err(|_| "audio thread disconnected")?;
                    // Wait for the note duration
                    std::thread::sleep(std::time::Duration::from_secs_f64(beat_duration));
                }
                Event::Chord(notes) => {
                    let freqs: Vec<f64> = notes.iter().map(|n| n.note.to_freq(n.octave)).collect();
                    let desc: Vec<String> = notes
                        .iter()
                        .map(|n| format!("{:?}{}", n.note, n.octave))
                        .collect();
                    println!("  Playing chord [{}]", desc.join(" "));
                    cmd_tx
                        .send(AudioCommand::PlayChord {
                            freqs,
                            duration_secs: beat_duration,
                        })
                        .map_err(|_| "audio thread disconnected")?;
                    std::thread::sleep(std::time::Duration::from_secs_f64(beat_duration));
                }
                Event::Rest(beats) => {
                    let rest_duration = beat_duration * beats;
                    println!("  Rest ({} beats)", beats);
                    cmd_tx
                        .send(AudioCommand::Rest {
                            duration_secs: rest_duration,
                        })
                        .map_err(|_| "audio thread disconnected")?;
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
    let _ = cmd_tx.send(AudioCommand::Stop);

    Ok(())
}
