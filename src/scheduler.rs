//! Builds a sorted timeline of (beat, command) from a Song and loaded patterns.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::note::{Event, Pattern, event_duration};
use crate::synth::LiveCommand;

/// One scheduled event: at this beat, send this command.
#[derive(Debug)]
pub struct ScheduledEvent {
    pub beat: f64,
    pub command: LiveCommand,
}

/// Build a sorted list of (beat, command) for the entire song.
/// patterns: map from notes file path (as used in song) to loaded Pattern.
pub fn build_schedule(
    song: &crate::song::Song,
    patterns: &HashMap<PathBuf, Pattern>,
) -> Result<Vec<ScheduledEvent>, String> {
    let mut events: Vec<ScheduledEvent> = Vec::new();

    for (track_idx, track) in song.tracks.iter().enumerate() {
        let mut track_beat = 0.0_f64;
        let mut key_counter: u32 = 0;

        for segment in &track.sequence {
            let pattern = patterns.get(&segment.notes_path).ok_or_else(|| {
                format!(
                    "pattern not loaded: {}",
                    segment.notes_path.display()
                )
            })?;

            let pattern_len = pattern.length_beats();

            for _rep in 0..segment.times {
                let mut event_beat = 0.0_f64;

                for ev in &pattern.events {
                    match ev {
                        Event::Note(n) => {
                            // Use private-use codepoints for unique keys per voice
                            let key = char::from_u32(0xE000u32.saturating_add(key_counter % 0x200))
                                .unwrap_or('\0');
                            key_counter += 1;
                            let freq = n.note.to_freq(n.octave);
                            events.push(ScheduledEvent {
                                beat: track_beat + event_beat,
                                command: LiveCommand::NoteOn {
                                    track: track_idx,
                                    key,
                                    freq,
                                },
                            });
                            events.push(ScheduledEvent {
                                beat: track_beat + event_beat + 1.0,
                                command: LiveCommand::NoteOff {
                                    track: track_idx,
                                    key,
                                },
                            });
                        }
                        Event::Chord(notes) => {
                            for n in notes {
                                let key = char::from_u32(0xE000u32.saturating_add(key_counter % 0x200))
                                    .unwrap_or('\0');
                                key_counter += 1;
                                let freq = n.note.to_freq(n.octave);
                                events.push(ScheduledEvent {
                                    beat: track_beat + event_beat,
                                    command: LiveCommand::NoteOn {
                                        track: track_idx,
                                        key,
                                        freq,
                                    },
                                });
                                events.push(ScheduledEvent {
                                    beat: track_beat + event_beat + 1.0,
                                    command: LiveCommand::NoteOff {
                                        track: track_idx,
                                        key,
                                    },
                                });
                            }
                        }
                        Event::Rest(_) | Event::BarLine => {}
                    }
                    event_beat += event_duration(ev);
                }

                track_beat += pattern_len;
            }
        }
    }

    events.sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap_or(std::cmp::Ordering::Equal));
    Ok(events)
}
