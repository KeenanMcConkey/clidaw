# CLIDAW - Command-Line Digital Audio Workspace

A minimalist, text-based digital audio workstation that embraces the Unix philosophy: do one thing well, compose tools together, and make everything human-readable and version-controllable.

## Core Philosophy

Music production should be:
- **Text-based**: Edit your compositions in any text editor
- **Version-controllable**: Track changes with git
- **Composable**: Pipe outputs between tools
- **Accessible**: No expensive hardware or GUI required
- **Portable**: Run on any machine with a terminal

## Features

### Current Implementation (v0.1)

- ✅ **Text-based note notation** - Write music using keyboard characters
- ✅ **Note patterns (.notes)** - Fixed-length patterns (e.g. one bar) with optional `beats` and `loop`
- ✅ **Songs (.song)** - Combine multiple instruments with sequences of patterns (e.g. verse×4, chorus×4)
- ✅ **Instruments (.instr)** - Per-instrument ADSR envelope (attack, decay, sustain, release)
- ✅ **Multi-track playback** - Multiple instruments play in parallel from a single .song file
- ✅ **Live keyboard mode** - Play notes in real-time by typing
- ✅ **Chord support** - Play multiple notes simultaneously
- ✅ **ADSR envelopes** - Per-voice envelope for natural note shape
- ✅ **Sine wave synthesis** - Basic audio synthesis engine

### Planned Features

- [ ] Multiple waveform types (square, sawtooth, triangle)
- [ ] Basic filter implementation (low-pass, high-pass)
- [ ] WAV file rendering
- [ ] MIDI input/output
- [ ] Effect chain system
- [ ] Visual waveform display (ASCII art)
- [ ] Sample playback

## How It Works

### File Types

| File       | Purpose |
|-----------|---------|
| **.notes** | A single *pattern*: a set of notes over a fixed number of beats (e.g. one bar). Can specify `beats` and `loop`. |
| **.instr** | An *instrument*: ADSR envelope (attack, decay, sustain, release) in seconds. |
| **.song**  | A *song*: lists instruments and, per instrument, a sequence of patterns with repeat counts (e.g. `verse.notes * 4` then `chorus.notes * 4`). |

### Note Pattern Format (.notes)

Patterns are written as text where characters represent musical notes. Each .notes file describes **one pattern** (typically one bar of your time signature).

#### Character Mapping

```
Keyboard Layout → Musical Notes

Home Row (Natural Notes):
a s d f g h j k  l  ;  '
C D E F G A B C  D  E  F

Top Row (Sharps/Flats):
w  e     t  y  u     o  p
C# D#    F# G# A#    C# D#

Numbers 1-8: Set octave
Space/Tab:   Ignored (for formatting)
-:           Rest
|:           Bar line (visual marker)
[...]:       Chord (multiple notes together)
```

#### Pattern Directives

- `beats: <n>` - Length of this pattern in beats (e.g. 4 for one 4/4 bar). If omitted, computed from events.
- `loop: true|false` - Whether this pattern loops (for display/editor use; playback repeat is set in .song).
- `time_signature: <num>/<den>` - Time signature (default: 4/4)
- `octave: <0-8>` - Default octave (default: 4)

#### Example Pattern (`verse.notes`)

```
# Verse: one bar (4 beats)
beats: 4
loop: false
octave: 3

a --- a --- | f --- f --- |
```

#### Example Pattern with Chords (`demo.notes`)

```
beats: 4
loop: false
octave: 4

a s d f g h j k |
[adg] [fhk] [gdl] [adg]
```

### Instrument Format (.instr)

Instruments define the ADSR envelope (times in seconds, sustain 0–1):

```
attack: 0.01
decay: 0.1
sustain: 0.7
release: 0.25
```

### Song Format (.song)

A song ties instruments to sequences of patterns. Paths are relative to the .song file.

```
tempo: 120
time_signature: 4/4

instrument: pluck.instr
verse.notes * 4
chorus.notes * 4

instrument: pad.instr
melody.notes * 8
```

- **First instrument** plays `verse.notes` 4 times, then `chorus.notes` 4 times.
- **Second instrument** plays `melody.notes` 8 times.
- All tracks run in parallel; tempo and time signature apply to the whole song.

### Event Types (within a pattern)

- **Note**: Single note (e.g., `a`, `w`, `j`)
- **Chord**: Multiple notes in brackets (e.g., `[ace]`, `[adg]`)
- **Rest**: One or more dashes (e.g., `-`, `---`)
- **Bar Line**: Visual separator `|` (no timing impact)

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- ALSA development libraries (Linux)
  ```bash
  # Ubuntu/Debian
  sudo apt-get install libasound2-dev

  # Fedora
  sudo dnf install alsa-lib-devel

  # Arch
  sudo pacman -S alsa-lib
  ```

### From Source

```bash
git clone https://github.com/KeenanMcConkey/clidaw
cd clidaw
cargo build --release
cargo install --path .
```

## Usage

### Play a Song (.song file)

Play a full song (multiple instruments, each with a sequence of patterns):

```bash
clidaw play examples/demo.song
```

Override tempo:
```bash
clidaw play my.song --tempo 140
```

### Play a Single Pattern (.notes file)

Play one pattern once (default tempo 120):

```bash
clidaw play examples/demo.notes
```

Use a specific instrument and tempo:
```bash
clidaw play examples/demo.notes --instrument examples/pluck.instr --tempo 130
```

### Live Keyboard Mode

Launch interactive mode and play notes by typing:

```bash
clidaw live
```

**Controls:**
- Type keyboard keys (`a-l`, `;`, `'`, `w`, `e`, `t`, `y`, `u`, `o`, `p`) to play notes
- Number keys `1-8` to change octave
- `Esc` to quit

### Parse and Inspect

View the parsed structure of a .notes pattern:

```bash
clidaw parse examples/verse.notes
```

This displays:
- Pattern length (beats) and loop flag
- Time signature and default octave
- All events with note names and frequencies

## Example Workflow

### Quick Pattern

```bash
# Create a one-bar pattern
cat > quick.notes << 'EOF'
beats: 4
loop: false
octave: 4
a s d f g a j
EOF

# Play it (once)
clidaw play quick.notes

# Play it with a pluck instrument
clidaw play quick.notes --instrument examples/pluck.instr
```

### Song with Multiple Instruments

```bash
# Create patterns (one bar each)
# verse.notes, chorus.notes, melody.notes

# Define a song: bass plays verse 4x then chorus 4x; lead plays melody 8x
cat > my.song << 'EOF'
tempo: 120
time_signature: 4/4

instrument: pluck.instr
verse.notes * 4
chorus.notes * 4

instrument: pad.instr
melody.notes * 8
EOF

# Play the song (all tracks in parallel)
clidaw play my.song
```

## Technical Architecture

### Components

```
┌─────────────────┐
│  Parser         │  parse_pattern() → Pattern (beats, loop, events)
│  (parser.rs)    │
└────────┬────────┘
         │
┌────────▼────────┐
│  Note Model     │  Pattern, Event, NoteEvent; event_duration, pattern length
│  (note.rs)      │
└────────┬────────┘
         │
┌────────▼────────┐     ┌─────────────────┐
│  Song Loader    │     │  Instrument     │  .instr → ADSR
│  (song.rs)      │     │  (instrument.rs)│
└────────┬────────┘     └────────┬────────┘
         │                       │
┌────────▼────────┐     ┌────────▼────────┐
│  Scheduler     │     │  Synth Engine   │  Multi-track; one ADSR per track
│  (scheduler.rs)│────▶│  (synth.rs)     │  play_schedule(), play_pattern()
└─────────────────┘     └────────┬────────┘
                                │
                        ┌───────▼────────┐
                        │  Audio Backend │  cpal (ALSA, CoreAudio, WASAPI)
                        │  (cpal)        │
                        └────────────────┘
```

### Technology Stack

- **Language**: Rust 2024 edition
- **Audio I/O**: [cpal](https://github.com/RustAudio/cpal) - Cross-platform audio library
- **CLI Parsing**: [clap](https://github.com/clap-rs/clap) - Command-line argument parser
- **Terminal UI**: [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation (for live mode)

### Code Structure

```
src/
├── main.rs       - CLI; play .song / .notes, parse, live
├── note.rs       - Pattern, Event, NoteEvent; event_duration
├── parser.rs     - parse_pattern() for .notes, parse() (legacy)
├── song.rs       - Song, SongTrack, Segment; load .song
├── instrument.rs - Instrument, load .instr → ADSR
├── scheduler.rs  - build_schedule(song, patterns) → sorted (beat, command)
├── synth.rs      - AudioEngine (single or multi-track), play_schedule, play_pattern
└── repl.rs       - Interactive live keyboard mode

examples/
├── demo.notes    - Single pattern (scale + chords)
├── verse.notes   - Bass pattern (verse)
├── chorus.notes  - Bass pattern (chorus)
├── melody.notes  - Lead pattern
├── demo.song     - Song: bass (verse×4, chorus×4), lead (melody×8)
├── pluck.instr   - Short pluck ADSR
└── pad.instr     - Pad/strings ADSR
```

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Examples

```bash
# Play the demo song (multi-track)
cargo run -- play examples/demo.song

# Play a single pattern
cargo run -- play examples/demo.notes
cargo run -- play examples/demo.notes --instrument examples/pluck.instr

# Inspect a pattern
cargo run -- parse examples/verse.notes

# Live keyboard
cargo run -- live
```

## Contributing

Contributions are welcome! Areas of interest:

- **DSP Features**: Additional waveforms, filters, effects
- **File Format**: Extended notation, pattern/song format improvements
- **Synthesis**: Additional waveforms, LFOs, filters, modulation
- **Platform Support**: Testing and fixes for different operating systems
- **Documentation**: Examples, tutorials, use cases
- **Performance**: Optimization opportunities

Please open an issue to discuss major changes before starting work.

## Roadmap

### v0.2 - Enhanced Synthesis
- Multiple oscillator waveforms
- Basic low-pass filter
- WAV file export

### v0.3 - Effects & Polish  
- Reverb and delay effects
- Better live mode UI
- Configuration file support
- More example songs and patterns

### v1.0 - Full-Featured DAW
- Complete modular synthesis
- MIDI support
- Multi-track mixing and panning
- Sample playback
- Pattern sequencer

## License

MIT License - See LICENSE file for details

## Inspiration

This project draws inspiration from:

- **Sonic Pi** - Live coding music environment
- **Pure Data / Max/MSP** - Modular synthesis
- **Vim** - Modal editing and text-based workflow  
- **Unix Philosophy** - Composable, focused tools
- **Markdown** - Human-readable plain text format

## Project Status

**Status**: Early development (v0.1)  
**Stability**: Experimental - API may change

This is a hobby project exploring text-based music creation. The goal is to create a fun, accessible tool for making music through typing. Contributions and feedback are very welcome!

---

**Repository**: https://github.com/KeenanMcConkey/clidaw
