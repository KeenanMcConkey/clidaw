# CLI DAW - Command-Line Digital Audio Workspace

## Project Overview

A minimalist, text-based digital audio workstation that embraces the Unix philosophy: do one thing well, compose tools together, and make everything human-readable and version-controllable.

## Core Philosophy

Music production should be:
- **Text-based**: Edit your compositions in any text editor
- **Version-controllable**: Track changes with git
- **Composable**: Pipe outputs between tools
- **Accessible**: No expensive hardware or GUI required
- **Portable**: Run on any machine with a terminal

## How It Works

### Note Files

Compositions are written as simple text files where characters represent musical notes. The system maps keyboard characters to musical notes, making it intuitive to "play" by typing.

#### Character Mapping

```
Keyboard Layout → Musical Notes

a s d f g h j k l ; '
C D E F G A B C D E F

w e   t y u   o p
C# D#  F# G# A#  C# D#

Numbers (1-8) control octave
Space = rest
| = bar line
```

#### Example Composition File (`song.notes`)

```
# My First Song
tempo: 120
octave: 4

# Melody
a a a f | j j j - |
h h h g | f f f - |

# Chord progression
[aeg] [sdh] [dfj] [aeg]
```

### Modular Synth Architecture

The audio engine simulates a modular synthesizer with patchable components:

#### Core Modules

1. **Oscillators**
   - Sine, Square, Saw, Triangle waves
   - Multiple oscillators per voice
   - Pitch, phase, and pulse-width controls

2. **Filters**
   - Low-pass, High-pass, Band-pass
   - Resonance and cutoff frequency
   - Envelope followers

3. **Envelopes**
   - ADSR (Attack, Decay, Sustain, Release)
   - Assignable to any parameter
   - Multiple envelopes per voice

4. **Effects**
   - Reverb, Delay, Chorus, Distortion
   - Chain-able effect racks
   - Per-track or master bus

5. **LFOs (Low-Frequency Oscillators)**
   - Modulate any parameter
   - Multiple waveforms
   - Sync to tempo

#### Patch Files

Synthesizer configurations are stored as `.patch` files:

```yaml
# bass.patch
name: "Deep Bass"

oscillators:
  - type: saw
    octave: -1
  - type: square
    octave: -1
    detune: -7

filter:
  type: lowpass
  cutoff: 200
  resonance: 0.3
  envelope: filter_env

envelopes:
  amp_env:
    attack: 0.01
    decay: 0.2
    sustain: 0.7
    release: 0.5
  filter_env:
    attack: 0.01
    decay: 0.3
    sustain: 0.2
    release: 0.4

effects:
  - type: distortion
    drive: 0.3
  - type: reverb
    size: 0.2
    mix: 0.1
```

## Technical Architecture

### Components

```
┌─────────────────┐
│  Note Parser    │  Reads .notes files, converts to MIDI-like events
└────────┬────────┘
         │
┌────────▼────────┐
│  Synth Engine   │  Loads .patch files, generates audio
└────────┬────────┘
         │
┌────────▼────────┐
│  Audio Backend  │  Outputs to speakers/file (PortAudio/JACK)
└─────────────────┘
```

### Command-Line Interface

```bash
# Play a composition
clidaw play song.notes --patch bass.patch

# Render to audio file
clidaw render song.notes -o output.wav --tempo 140

# Live coding mode (auto-reload on file change)
clidaw live song.notes --patch synth.patch

# List available patches
clidaw patches list

# Create new patch from template
clidaw patches new mysynth --template poly

# Interactive patch editor
clidaw patch-editor bass.patch
```

## Features

### Current Goals (v0.1)

- [x] Basic note parser (characters → note events)
- [x] Single oscillator playback
- [ ] ADSR envelope support
- [ ] Basic filter implementation
- [ ] WAV file rendering
- [X] Real-time playback

### Planned Features (v0.2+)

- [ ] Multiple simultaneous tracks
- [ ] MIDI input/output support
- [ ] Visual waveform display (ASCII art)
- [ ] Built-in metronome
- [ ] Loop and sequence support
- [ ] Polyphonic synthesizer
- [ ] Effect chain system
- [ ] Live parameter automation
- [ ] Sample playback (drum machine mode)

### Future Vision (v1.0+)

- [ ] Plugin system for custom modules
- [ ] Network collaboration (shared editing)
- [ ] Pattern library and sharing
- [ ] Algorithmic composition tools
- [ ] Integration with existing DAWs (ReaScript, OSC)
- [ ] Mobile companion app for remote control
- [ ] Visual patch cable editor (optional GUI)

## Example Workflows

### Quick Sketch

```bash
# Write a quick melody
echo "a s d f g a j" > idea.notes

# Play it with a default patch
clidaw play idea.notes
```

### Composition Workflow

```bash
# Create project structure
mkdir my-song
cd my-song

# Write parts
vim melody.notes
vim bass.notes
vim drums.notes

# Create custom patches
clidaw patches new lead --template mono
vim lead.patch

# Arrange and render
clidaw arrange parts/*.notes -o arrangement.wav
```

### Live Performance

```bash
# Start live mode with hot-reloading
clidaw live performance.notes --patch live.patch

# Edit the file in another terminal
# Changes apply immediately to playback
```

## File Format Specifications

### .notes File Format

```
# Comments start with #
tempo: 120           # BPM
time_signature: 4/4  # Time signature
octave: 4            # Default octave (0-8)
patch: bass.patch    # Default patch file

# Track definitions
[track: melody]
patch: lead.patch
a a a f | j j j - |

[track: bass]
octave: 2
a --- a --- | f --- f --- |

# Chords use brackets
[ace] [dfj] [egk]

# Rests
- (short rest)
--- (longer rest, triplet)
```

### .patch File Format (YAML)

See example above. Supports:
- Oscillator configuration (waveform, pitch, detune)
- Filter parameters (type, cutoff, resonance)
- Envelope generators (ADSR values)
- Effect chains (type and parameters)
- LFO routing and modulation

### .session File Format (Song Arrangement)

```yaml
name: "My Song"
tempo: 128
tracks:
  - name: "Lead"
    notes: melody.notes
    patch: lead.patch
    volume: 0.8
    pan: 0.5
  - name: "Bass"
    notes: bass.notes
    patch: bass.patch
    volume: 1.0
    pan: 0.5
  - name: "Drums"
    samples: drums.yml
    volume: 0.9
    pan: 0.5
```

## Technical Stack

- **Language**: Rust (performance, safety, cross-platform)
- **Audio**: cpal or PortAudio (cross-platform audio I/O)
- **DSP**: fundsp or custom DSP library
- **Synthesis**: Custom modular engine
- **CLI**: clap (command-line parsing)
- **File Format**: YAML for configs, custom text format for notes
- **Testing**: Audio snapshots, unit tests for DSP

## Installation

```bash
# From source
git clone https://github.com/yourusername/clidaw
cd clidaw
cargo build --release
cargo install --path .

# With package manager (future)
brew install clidaw
cargo install clidaw
```

## Configuration

User configuration stored in `~/.config/clidaw/config.yml`:

```yaml
audio:
  sample_rate: 48000
  buffer_size: 512
  device: default

midi:
  input_device: "MIDI Controller"

paths:
  patches: "~/.config/clidaw/patches"
  samples: "~/.config/clidaw/samples"

defaults:
  tempo: 120
  octave: 4
  patch: "default.patch"
```

## Contributing

This project welcomes contributions! Areas of interest:
- DSP algorithm implementations
- New synthesizer modules
- File format improvements
- Documentation and examples
- Platform-specific optimizations

## License

MIT License (or GPL if using GPL-licensed audio libraries)

## Inspiration

- Sonic Pi (live coding music environment)
- Pure Data / Max/MSP (modular synthesis)
- Vim (modal editing, text-based workflow)
- Unix philosophy (composable tools)
- Markdown (readable plain text format)

---

**Status**: Early concept phase
**Looking for**: Contributors, feedback, ideas
**Contact**: [your-email@example.com]
