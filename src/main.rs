mod note;
mod parser;
mod repl;
mod synth;

use clap::{Parser, Subcommand};
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
    /// Play a composition through speakers
    Play {
        /// Path to a .notes file
        file: PathBuf,

        /// Override tempo (BPM)
        #[arg(long)]
        tempo: Option<u32>,
    },

    /// Parse a .notes file and display its events
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
        Command::Play { file, tempo } => {
            let input = read_file(&file);
            let mut comp = parse_input(&input);

            if let Some(t) = tempo {
                comp.tempo = t;
            }

            println!(
                "Playing: {} BPM, {}/{} time",
                comp.tempo, comp.time_signature.0, comp.time_signature.1
            );
            println!();

            if let Err(e) = synth::play(&comp) {
                eprintln!("Playback error: {}", e);
                std::process::exit(1);
            }
        }
        Command::Parse { file } => {
            let input = read_file(&file);
            let comp = parse_input(&input);
            print_composition(&comp);
        }
        Command::Live => {
            if let Err(e) = repl::run() {
                eprintln!("Live mode error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn read_file(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path.display(), e);
        std::process::exit(1);
    })
}

fn parse_input(input: &str) -> note::Composition {
    parser::parse(input).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    })
}

fn print_composition(comp: &note::Composition) {
    println!("Tempo: {} BPM", comp.tempo);
    println!(
        "Time signature: {}/{}",
        comp.time_signature.0, comp.time_signature.1
    );
    println!("Tracks: {}", comp.tracks.len());
    println!();
    for track in &comp.tracks {
        println!("--- Track: {} ---", track.name);
        for event in &track.events {
            match event {
                note::Event::Note(n) => {
                    println!(
                        "  {:?} (octave {}, {:.1} Hz)",
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
                note::Event::BarLine => {
                    println!("  |");
                }
            }
        }
        println!();
    }
}
