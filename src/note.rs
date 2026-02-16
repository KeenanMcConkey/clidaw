/// Musical note names (chromatic scale)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteName {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl NoteName {
    /// MIDI note number within an octave (C=0, B=11)
    pub fn semitone(self) -> u8 {
        match self {
            NoteName::C => 0,
            NoteName::CSharp => 1,
            NoteName::D => 2,
            NoteName::DSharp => 3,
            NoteName::E => 4,
            NoteName::F => 5,
            NoteName::FSharp => 6,
            NoteName::G => 7,
            NoteName::GSharp => 8,
            NoteName::A => 9,
            NoteName::ASharp => 10,
            NoteName::B => 11,
        }
    }

    /// Convert to MIDI note number given an octave (0-8)
    /// Middle C (C4) = MIDI 60
    pub fn to_midi(self, octave: u8) -> u8 {
        (octave + 1) * 12 + self.semitone()
    }

    /// Frequency in Hz (A4 = 440 Hz)
    pub fn to_freq(self, octave: u8) -> f64 {
        let midi = self.to_midi(octave) as f64;
        440.0 * 2.0_f64.powf((midi - 69.0) / 12.0)
    }
}

/// A single note event
#[derive(Debug, Clone, PartialEq)]
pub struct NoteEvent {
    pub note: NoteName,
    pub octave: u8,
}

/// An event in the composition timeline
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A single note
    Note(NoteEvent),
    /// Multiple notes sounding together
    Chord(Vec<NoteEvent>),
    /// A rest (duration in beats)
    Rest(f64),
    /// A bar line (visual/structural marker)
    BarLine,
}

/// A named track with its own settings and events
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Track {
    pub name: String,
    pub patch: Option<String>,
    pub octave: u8,
    pub events: Vec<Event>,
}

/// A full parsed composition
#[derive(Debug, Clone)]
pub struct Composition {
    pub tempo: u32,
    pub time_signature: (u8, u8),
    pub default_octave: u8,
    pub default_patch: Option<String>,
    pub tracks: Vec<Track>,
}

impl Composition {
    pub fn new() -> Self {
        Self {
            tempo: 120,
            time_signature: (4, 4),
            default_octave: 4,
            default_patch: None,
            tracks: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middle_c_midi() {
        assert_eq!(NoteName::C.to_midi(4), 60);
    }

    #[test]
    fn test_a4_frequency() {
        let freq = NoteName::A.to_freq(4);
        assert!((freq - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_semitones() {
        assert_eq!(NoteName::C.semitone(), 0);
        assert_eq!(NoteName::B.semitone(), 11);
    }
}
