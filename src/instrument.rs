//! Instrument definitions loaded from `.instr` files.
//!
//! An instrument file defines ADSR envelope parameters used during playback.
//! Paths in `.song` files reference these instruments.

use std::fs;
use std::path::Path;

/// Instrument definition (ADSR envelope parameters).
/// Load from a `.instr` file and convert to `synth::Adsr` for playback.
#[derive(Debug, Clone)]
pub struct Instrument {
    /// Attack time in seconds (0 → peak)
    pub attack: f64,
    /// Decay time in seconds (peak → sustain level)
    pub decay: f64,
    /// Sustain level (0.0..=1.0) while key is held
    pub sustain: f64,
    /// Release time in seconds (current level → 0 after key release)
    pub release: f64,
}

impl Default for Instrument {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.25,
        }
    }
}

/// Parse a single "key: value" line. Returns (key, value) or None.
fn parse_line(line: &str) -> Option<(&str, f64)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let colon = trimmed.find(':')?;
    let key = trimmed[..colon].trim();
    let value = trimmed[colon + 1..].trim().parse::<f64>().ok()?;
    Some((key, value))
}

/// Load an instrument from a `.instr` file.
///
/// Format (one per line, optional comments with #):
/// ```text
/// # ADSR envelope (times in seconds, sustain 0..1)
/// attack: 0.01
/// decay: 0.1
/// sustain: 0.7
/// release: 0.25
/// ```
pub fn load(path: &Path) -> Result<Instrument, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("reading instrument file: {}", e))?;

    let mut attack = None;
    let mut decay = None;
    let mut sustain = None;
    let mut release = None;

    for (line_num, line) in content.lines().enumerate() {
        let (key, value) = match parse_line(line) {
            Some(p) => p,
            None => continue,
        };
        match key {
            "attack" => attack = Some(value),
            "decay" => decay = Some(value),
            "sustain" => sustain = Some(value),
            "release" => release = Some(value),
            _ => {
                return Err(format!(
                    "unknown key '{}' at line {}",
                    key,
                    line_num + 1
                ));
            }
        }
    }

    Ok(Instrument {
        attack: attack.unwrap_or(0.01),
        decay: decay.unwrap_or(0.1),
        sustain: sustain.unwrap_or(0.7).clamp(0.0, 1.0),
        release: release.unwrap_or(0.25),
    })
}

impl Instrument {
    /// Convert to the synth's ADSR type (used when creating the audio engine).
    pub fn to_adsr(&self) -> crate::synth::Adsr {
        crate::synth::Adsr {
            attack: self.attack,
            decay: self.decay,
            sustain: self.sustain,
            release: self.release,
        }
    }
}
