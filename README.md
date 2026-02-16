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
- ✅ **Note parser** - Converts characters to musical events
- ✅ **Live keyboard mode** - Play notes in real-time by typing
- ✅ **File playback** - Play .notes files through speakers
- ✅ **Multiple tracks** - Support for multi-track compositions
- ✅ **Chord support** - Play multiple notes simultaneously
- ✅ **Metadata control** - Tempo, time signature, and octave settings
- ✅ **Sine wave synthesis** - Basic audio synthesis engine

### Planned Features

- [ ] ADSR envelope support
- [ ] Multiple waveform types (square, sawtooth, triangle)
- [ ] Basic filter implementation (low-pass, high-pass)
- [ ] WAV file rendering
- [ ] MIDI input/output
- [ ] Effect chain system
- [ ] Visual waveform display (ASCII art)
- [ ] Sample playback

## How It Works

### Note File Format

Compositions are written as simple text files where characters represent musical notes. The keyboard layout maps intuitively to a piano-like interface:

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

#### Example Composition

Create a file `song.notes`:

```
# My First Song
tempo: 120
octave: 4

# C major scale
a s d f g h j k |

# Chord progression (C - F - G - C)
[adg] [fhk] [gdl] [adg]

# With rests
a s d - f - g - |

# Multiple tracks
[track: melody]
a a a f | j j j - |

[track: bass]
octave: 2
a --- a --- | f --- f --- |
```

### Metadata Directives

- `tempo: <bpm>` - Set tempo in beats per minute (default: 120)
- `time_signature: <num>/<den>` - Set time signature (default: 4/4)
- `octave: <0-8>` - Set default octave (default: 4)
- `patch: <filename>` - Specify patch file (planned feature)
- `[track: <name>]` - Start a new named track

### Event Types

- **Note**: Single note (e.g., `a`, `w`, `j`)
- **Chord**: Multiple notes in brackets (e.g., `[ace]`, `[adg]`)
- **Rest**: One or more dashes (e.g., `-`, `---` for longer rest)
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

### Play a Composition File

```bash
clidaw play examples/demo.notes
```

Override tempo:
```bash
clidaw play song.notes --tempo 140
```

### Live Keyboard Mode

Launch interactive mode where you can play notes by typing:

```bash
clidaw live
```

**Controls:**
- Type keyboard keys (`a-l`, `;`, `'`, `w`, `e`, `t`, `y`, `u`, `o`, `p`) to play notes
- Number keys `1-8` to change octave
- `Esc` to quit

### Parse and Inspect

View the parsed structure of a .notes file:

```bash
clidaw parse song.notes
```

This displays:
- Tempo and time signature
- Number of tracks
- All events with note names, frequencies, and types

## Example Workflow

### Quick Melody

```bash
# Create a simple melody
cat > quick.notes << 'EOF'
tempo: 130
octave: 4
a s d f g a j
EOF

# Play it
clidaw play quick.notes
```

### Multi-Track Composition

```bash
# Create a more complex arrangement
cat > song.notes << 'EOF'
tempo: 120
time_signature: 4/4

[track: melody]
octave: 5
a a a f | j j j - |
h h h g | f f f - |

[track: bass]
octave: 2
a --- a --- | f --- f --- |
a --- a --- | f --- f --- |

[track: chords]
octave: 3
[adg] --- --- --- | [fhk] --- --- --- |
[gdl] --- --- --- | [adg] --- --- --- |
EOF

# Play the full composition
clidaw play song.notes
```

## Technical Architecture

### Components

```
┌─────────────────┐
│  Note Parser    │  Parses .notes files into structured events
│  (parser.rs)    │
└────────┬────────┘
         │
┌────────▼────────┐
│  Note Model     │  Represents notes, chords, tracks, compositions
│  (note.rs)      │
└────────┬────────┘
         │
┌────────▼────────┐
│  Synth Engine   │  Generates audio from note events
│  (synth.rs)     │  - Real-time synthesis (live mode)
│                 │  - Event playback (file mode)
└────────┬────────┘
         │
┌────────▼────────┐
│  Audio Backend  │  Cross-platform audio output via cpal
│  (cpal)         │  Supports ALSA (Linux), CoreAudio (macOS), WASAPI (Windows)
└─────────────────┘
```

### Technology Stack

- **Language**: Rust 2024 edition
- **Audio I/O**: [cpal](https://github.com/RustAudio/cpal) - Cross-platform audio library
- **CLI Parsing**: [clap](https://github.com/clap-rs/clap) - Command-line argument parser
- **Terminal UI**: [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation (for live mode)

### Code Structure

```
src/
├── main.rs      - CLI interface and command routing
├── note.rs      - Note types and composition model
├── parser.rs    - Text parser for .notes files
├── synth.rs     - Audio synthesis engine
└── repl.rs      - Interactive live keyboard mode

examples/
└── demo.notes   - Example composition file
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
cargo run -- play examples/demo.notes
cargo run -- live
cargo run -- parse examples/demo.notes
```

## Contributing

Contributions are welcome! Areas of interest:

- **DSP Features**: Additional waveforms, filters, effects
- **File Format**: Extended notation features, pattern support
- **Synthesis**: Envelopes, LFOs, modulation
- **Platform Support**: Testing and fixes for different operating systems
- **Documentation**: Examples, tutorials, use cases
- **Performance**: Optimization opportunities

Please open an issue to discuss major changes before starting work.

## Roadmap

### v0.2 - Enhanced Synthesis
- Multiple oscillator waveforms
- ADSR envelopes
- Basic low-pass filter
- WAV file export

### v0.3 - Effects & Polish  
- Reverb and delay effects
- Better live mode UI
- Configuration file support
- More example compositions

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
