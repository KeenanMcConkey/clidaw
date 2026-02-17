//! Song definitions: multiple instruments, each with a sequence of .notes patterns.
//!
//! A `.song` file lists instruments (.instr) and then per-track sequences of
//! (notes_file, repeat_count) to build the full song.

use std::fs;
use std::path::{Path, PathBuf};

/// One segment in a track: play this pattern N times.
#[derive(Debug, Clone)]
pub struct Segment {
    pub notes_path: PathBuf,
    pub times: u32,
}

/// One track: one instrument + a sequence of (pattern, repeat count).
#[derive(Debug, Clone)]
pub struct SongTrack {
    pub instrument_path: PathBuf,
    pub sequence: Vec<Segment>,
}

/// A song: tempo, time signature, and one or more tracks (instrument + pattern sequence).
#[derive(Debug, Clone)]
pub struct Song {
    pub tempo: u32,
    pub time_signature: (u8, u8),
    pub tracks: Vec<SongTrack>,
}

fn parse_kv(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let colon = trimmed.find(':')?;
    let key = trimmed[..colon].trim();
    let value = trimmed[colon + 1..].trim();
    Some((key, value))
}

/// Parse "file.notes * 4" or "file.notes" (times = 1)
fn parse_sequence_line(line: &str) -> Option<(String, u32)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (path, times) = if let Some((left, right)) = trimmed.split_once('*') {
        let path = left.trim();
        let times = right.trim().parse::<u32>().unwrap_or(1);
        (path, times)
    } else {
        (trimmed, 1)
    };
    if path.is_empty() {
        return None;
    }
    Some((path.to_string(), times))
}

/// Load a song from a `.song` file.
///
/// Format:
/// ```text
/// tempo: 120
/// time_signature: 4/4
/// instrument: bass.instr
/// verse.notes * 4
/// chorus.notes * 4
/// instrument: lead.instr
/// melody.notes * 8
/// ```
/// Paths are relative to the directory containing the .song file.
pub fn load(song_path: &Path) -> Result<Song, String> {
    let content = fs::read_to_string(song_path)
        .map_err(|e| format!("reading song file: {}", e))?;

    let base = song_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let mut tempo = 120u32;
    let mut time_signature = (4u8, 4u8);
    let mut tracks: Vec<SongTrack> = Vec::new();
    let mut current_instrument: Option<PathBuf> = None;
    let mut current_sequence: Vec<Segment> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if let Some((key, value)) = parse_kv(line) {
            match key {
                "tempo" => {
                    tempo = value.parse().map_err(|_| {
                        format!("invalid tempo '{}' at line {}", value, line_num + 1)
                    })?;
                }
                "time_signature" => {
                    let parts: Vec<&str> = value.split('/').collect();
                    if parts.len() == 2 {
                        let num: u8 = parts[0].trim().parse().map_err(|_| {
                            format!("invalid time_signature at line {}", line_num + 1)
                        })?;
                        let den: u8 = parts[1].trim().parse().map_err(|_| {
                            format!("invalid time_signature at line {}", line_num + 1)
                        })?;
                        time_signature = (num, den);
                    }
                }
                "instrument" => {
                    if let Some(inst) = current_instrument.take() {
                        if !current_sequence.is_empty() {
                            tracks.push(SongTrack {
                                instrument_path: inst,
                                sequence: std::mem::take(&mut current_sequence),
                            });
                        }
                    }
                    current_instrument = Some(base.join(value));
                }
                _ => {}
            }
            continue;
        }

        if let Some((path, times)) = parse_sequence_line(line) {
            if current_instrument.is_some() {
                current_sequence.push(Segment {
                    notes_path: base.join(&path),
                    times,
                });
            } else {
                return Err(format!(
                    "line {}: sequence line '{}' before any 'instrument:'",
                    line_num + 1,
                    line.trim()
                ));
            }
        }
    }

    if let Some(inst) = current_instrument.take() {
        if !current_sequence.is_empty() {
            tracks.push(SongTrack {
                instrument_path: inst,
                sequence: current_sequence,
            });
        }
    }

    if tracks.is_empty() {
        return Err("song has no tracks (need 'instrument:' followed by 'file.notes * N' lines)".to_string());
    }

    Ok(Song {
        tempo,
        time_signature,
        tracks,
    })
}
