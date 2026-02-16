use crate::note::*;

/// Map a keyboard character to a (NoteName, octave_offset) pair.
/// The octave_offset indicates notes that spill into the next octave
/// on the keyboard layout (k, l, ;, ', o, p).
pub fn char_to_note(c: char) -> Option<(NoteName, u8)> {
    match c {
        // Home row: natural notes
        'a' => Some((NoteName::C, 0)),
        's' => Some((NoteName::D, 0)),
        'd' => Some((NoteName::E, 0)),
        'f' => Some((NoteName::F, 0)),
        'g' => Some((NoteName::G, 0)),
        'h' => Some((NoteName::A, 0)),
        'j' => Some((NoteName::B, 0)),
        'k' => Some((NoteName::C, 1)),
        'l' => Some((NoteName::D, 1)),
        ';' => Some((NoteName::E, 1)),
        '\'' => Some((NoteName::F, 1)),

        // Top row: sharps/flats
        'w' => Some((NoteName::CSharp, 0)),
        'e' => Some((NoteName::DSharp, 0)),
        't' => Some((NoteName::FSharp, 0)),
        'y' => Some((NoteName::GSharp, 0)),
        'u' => Some((NoteName::ASharp, 0)),
        'o' => Some((NoteName::CSharp, 1)),
        'p' => Some((NoteName::DSharp, 1)),

        _ => None,
    }
}

/// Parse errors with location info
#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Parse a .notes file into a Composition
pub fn parse(input: &str) -> Result<Composition, ParseError> {
    let mut comp = Composition::new();
    let mut current_track_events: Vec<Event> = Vec::new();
    let mut current_track_name = String::from("default");
    let mut current_track_patch: Option<String> = None;
    let mut current_octave = comp.default_octave;

    for (line_idx, line) in input.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Metadata directives
        if let Some(value) = trimmed.strip_prefix("tempo:") {
            comp.tempo = value.trim().parse().map_err(|_| ParseError {
                line: line_num,
                message: format!("invalid tempo: {}", value.trim()),
            })?;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("time_signature:") {
            let parts: Vec<&str> = value.trim().split('/').collect();
            if parts.len() == 2 {
                let num: u8 = parts[0].parse().map_err(|_| ParseError {
                    line: line_num,
                    message: "invalid time signature numerator".into(),
                })?;
                let den: u8 = parts[1].parse().map_err(|_| ParseError {
                    line: line_num,
                    message: "invalid time signature denominator".into(),
                })?;
                comp.time_signature = (num, den);
            }
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("octave:") {
            let oct: u8 = value.trim().parse().map_err(|_| ParseError {
                line: line_num,
                message: format!("invalid octave: {}", value.trim()),
            })?;
            if oct > 8 {
                return Err(ParseError {
                    line: line_num,
                    message: "octave must be 0-8".into(),
                });
            }
            comp.default_octave = oct;
            current_octave = oct;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("patch:") {
            let patch = value.trim().to_string();
            if current_track_name == "default" && comp.tracks.is_empty() {
                comp.default_patch = Some(patch);
            } else {
                current_track_patch = Some(patch);
            }
            continue;
        }

        // Track header: [track: name]
        if trimmed.starts_with("[track:") && trimmed.ends_with(']') {
            // Save previous track if it has events
            if !current_track_events.is_empty() {
                comp.tracks.push(Track {
                    name: current_track_name.clone(),
                    patch: current_track_patch.take(),
                    octave: current_octave,
                    events: std::mem::take(&mut current_track_events),
                });
            }
            current_track_name = trimmed
                .strip_prefix("[track:")
                .unwrap()
                .strip_suffix(']')
                .unwrap()
                .trim()
                .to_string();
            current_octave = comp.default_octave;
            continue;
        }

        // Parse note line
        let events = parse_line(trimmed, current_octave, line_num)?;
        current_track_events.extend(events);
    }

    // Push final track
    if !current_track_events.is_empty() {
        comp.tracks.push(Track {
            name: current_track_name,
            patch: current_track_patch,
            octave: current_octave,
            events: current_track_events,
        });
    }

    Ok(comp)
}

/// Parse a single line of note text into events
fn parse_line(line: &str, octave: u8, _line_num: usize) -> Result<Vec<Event>, ParseError> {
    let mut events = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            // Whitespace: skip
            ' ' | '\t' => {
                chars.next();
            }

            // Bar line
            '|' => {
                chars.next();
                events.push(Event::BarLine);
            }

            // Rest: count consecutive dashes
            '-' => {
                let mut count = 0;
                while chars.peek() == Some(&'-') {
                    chars.next();
                    count += 1;
                }
                // Each dash = 1 beat of rest
                events.push(Event::Rest(count as f64));
            }

            // Chord: [notes]
            '[' => {
                chars.next(); // consume '['
                let mut chord_notes = Vec::new();
                while let Some(&inner) = chars.peek() {
                    if inner == ']' {
                        chars.next();
                        break;
                    }
                    if let Some((name, oct_offset)) = char_to_note(inner) {
                        chord_notes.push(NoteEvent {
                            note: name,
                            octave: octave.saturating_add(oct_offset),
                        });
                    }
                    chars.next();
                }
                if !chord_notes.is_empty() {
                    events.push(Event::Chord(chord_notes));
                }
            }

            // Note character
            _ => {
                if let Some((name, oct_offset)) = char_to_note(c) {
                    events.push(Event::Note(NoteEvent {
                        note: name,
                        octave: octave.saturating_add(oct_offset),
                    }));
                }
                // Unknown characters are silently skipped
                chars.next();
            }
        }
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_mapping() {
        assert_eq!(char_to_note('a'), Some((NoteName::C, 0)));
        assert_eq!(char_to_note('w'), Some((NoteName::CSharp, 0)));
        assert_eq!(char_to_note('k'), Some((NoteName::C, 1)));
        assert_eq!(char_to_note('z'), None);
    }

    #[test]
    fn test_parse_simple_melody() {
        let input = "tempo: 120\noctave: 4\n\na s d f";
        let comp = parse(input).unwrap();
        assert_eq!(comp.tempo, 120);
        assert_eq!(comp.default_octave, 4);
        assert_eq!(comp.tracks.len(), 1);

        let events = &comp.tracks[0].events;
        assert_eq!(events.len(), 4);
        assert_eq!(
            events[0],
            Event::Note(NoteEvent {
                note: NoteName::C,
                octave: 4
            })
        );
        assert_eq!(
            events[3],
            Event::Note(NoteEvent {
                note: NoteName::F,
                octave: 4
            })
        );
    }

    #[test]
    fn test_parse_rests_and_barlines() {
        let input = "a - | s";
        let comp = parse(input).unwrap();
        let events = &comp.tracks[0].events;
        assert_eq!(events.len(), 4);
        assert_eq!(events[1], Event::Rest(1.0));
        assert_eq!(events[2], Event::BarLine);
    }

    #[test]
    fn test_parse_long_rest() {
        let input = "a --- s";
        let comp = parse(input).unwrap();
        let events = &comp.tracks[0].events;
        assert_eq!(events[1], Event::Rest(3.0));
    }

    #[test]
    fn test_parse_chord() {
        // [adg] = C major chord (a=C, d=E, g=G)
        let input = "[adg]";
        let comp = parse(input).unwrap();
        let events = &comp.tracks[0].events;
        assert_eq!(events.len(), 1);
        if let Event::Chord(notes) = &events[0] {
            assert_eq!(notes.len(), 3);
            assert_eq!(notes[0].note, NoteName::C);
            assert_eq!(notes[1].note, NoteName::E);
            assert_eq!(notes[2].note, NoteName::G);
        } else {
            panic!("expected chord");
        }
    }

    #[test]
    fn test_parse_multiple_tracks() {
        let input = "\
[track: melody]
a s d f

[track: bass]
octave: 2
a --- a ---";
        let comp = parse(input).unwrap();
        assert_eq!(comp.tracks.len(), 2);
        assert_eq!(comp.tracks[0].name, "melody");
        assert_eq!(comp.tracks[1].name, "bass");
    }

    #[test]
    fn test_comments_ignored() {
        let input = "# this is a comment\na s d";
        let comp = parse(input).unwrap();
        assert_eq!(comp.tracks[0].events.len(), 3);
    }
}
