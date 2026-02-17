mod instrument;
mod note;
mod parser;
mod repl;
mod scheduler;
mod song;
mod synth;

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "clidaw", about = "Command-line digital audio workstation")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Play a .song file (multi-track) or a single .notes pattern
    Play {
        /// Path to a .song file or .notes file
        file: PathBuf,

        /// Instrument file (.instr); only used when playing a single .notes file
        #[arg(long)]
        instrument: Option<PathBuf>,

        /// Override tempo (BPM); for .notes or as override in .song
        #[arg(long)]
        tempo: Option<u32>,
    },

    /// Parse a .notes file and show pattern (beats, loop, events)
    Parse {
        /// Path to a .notes file
        file: PathBuf,
    },

    /// Interactive keyboard mode — play notes by typing
    Live,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Play {
            file,
            instrument: instrument_override,
            tempo,
        } => {
            if file
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("song"))
            {
                play_song(&file, tempo);
            } else {
                play_notes_file(&file, instrument_override, tempo);
            }
        }
        Command::Parse { file } => {
            let input = read_file(&file);
            let pattern = parser::parse_pattern(&input).unwrap_or_else(|e| {
                eprintln!("Parse error: {}", e);
                std::process::exit(1);
            });
            print_pattern(&pattern);
        }
        Command::Live => {
            if let Err(e) = repl::run() {
                eprintln!("Live mode error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn play_song(song_path: &PathBuf, tempo_override: Option<u32>) {
    let song = song::load(song_path).unwrap_or_else(|e| {
        eprintln!("Song error: {}", e);
        std::process::exit(1);
    });

    let tempo = tempo_override.unwrap_or(song.tempo);

    let mut adsrs = Vec::with_capacity(song.tracks.len());
    for track in &song.tracks {
        let adsr = instrument::load(&track.instrument_path)
            .unwrap_or_else(|e| {
                eprintln!(
                    "Instrument error {}: {}",
                    track.instrument_path.display(),
                    e
                );
                std::process::exit(1);
            })
            .to_adsr();
        adsrs.push(adsr);
    }

    let mut patterns: HashMap<std::path::PathBuf, note::Pattern> = HashMap::new();
    for track in &song.tracks {
        for seg in &track.sequence {
            if !patterns.contains_key(&seg.notes_path) {
                let content = fs::read_to_string(&seg.notes_path).unwrap_or_else(|e| {
                    eprintln!("Error reading {}: {}", seg.notes_path.display(), e);
                    std::process::exit(1);
                });
                let pattern = parser::parse_pattern(&content).unwrap_or_else(|e| {
                    eprintln!("Parse error in {}: {}", seg.notes_path.display(), e);
                    std::process::exit(1);
                });
                patterns.insert(seg.notes_path.clone(), pattern);
            }
        }
    }

    let schedule = scheduler::build_schedule(&song, &patterns).unwrap_or_else(|e| {
        eprintln!("Schedule error: {}", e);
        std::process::exit(1);
    });

    println!(
        "Playing song: {} BPM, {}/{} time, {} tracks, {} scheduled events",
        tempo,
        song.time_signature.0,
        song.time_signature.1,
        song.tracks.len(),
        schedule.len()
    );
    println!();

    let engine = synth::AudioEngine::with_instruments(adsrs).unwrap_or_else(|e| {
        eprintln!("Audio error: {}", e);
        std::process::exit(1);
    });

    if let Err(e) = synth::play_schedule(&schedule, tempo, &engine) {
        eprintln!("Playback error: {}", e);
        std::process::exit(1);
    }
}

fn play_notes_file(
    path: &PathBuf,
    instrument_override: Option<PathBuf>,
    tempo_override: Option<u32>,
) {
    let input = read_file(path);
    let pattern = parser::parse_pattern(&input).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });

    let tempo = tempo_override.unwrap_or(120);

    println!(
        "Playing pattern: {} beats, loop={}, {} BPM",
        pattern.length_beats(),
        pattern.loop_pattern,
        tempo
    );
    println!();

    let result = if let Some(instr_path) = instrument_override {
        let instr = instrument::load(&instr_path).unwrap_or_else(|e| {
            eprintln!("Instrument error: {}", e);
            std::process::exit(1);
        });
        let engine = synth::AudioEngine::with_adsr(instr.to_adsr()).unwrap_or_else(|e| {
            eprintln!("Audio error: {}", e);
            std::process::exit(1);
        });
        synth::play_pattern_with_engine(&pattern, tempo, &engine)
    } else {
        synth::play_pattern(&pattern, tempo)
    };

    if let Err(e) = result {
        eprintln!("Playback error: {}", e);
        std::process::exit(1);
    }
}

fn read_file(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path.display(), e);
        std::process::exit(1);
    })
}

fn print_pattern(pattern: &note::Pattern) {
    println!("Pattern: {} beats", pattern.length_beats());
    println!("Loop: {}", pattern.loop_pattern);
    println!("Time signature: {}/{}", pattern.time_signature.0, pattern.time_signature.1);
    println!("Octave: {}", pattern.default_octave);
    println!();
    for event in &pattern.events {
        match event {
            note::Event::Note(n) => {
                println!(
                    "  {:?}{} ({:.1} Hz)",
                    n.note,
                    n.octave,
                    n.note.to_freq(n.octave)
                );
            }
            note::Event::Chord(notes) => {
                let desc: Vec<String> = notes
                    .iter()
                    .map(|n| format!("{:?}{}", n.note, n.octave))
                    .collect();
                println!("  Chord [{}]", desc.join(" "));
            }
            note::Event::Rest(beats) => {
                println!(
                    "  Rest ({} beat{})",
                    beats,
                    if *beats != 1.0 { "s" } else { "" }
                );
            }
            note::Event::BarLine => println!("  |"),
        }
    }
}
