use std::io::{self, Write};
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use crate::parser::char_to_note;
use crate::synth::{AudioEngine, LiveCommand};

/// Run the interactive live keyboard mode
pub fn run() -> Result<(), String> {
    let engine = AudioEngine::new()?;

    let mut stdout = io::stdout();

    // Enter raw mode
    terminal::enable_raw_mode().map_err(|e| format!("failed to enable raw mode: {}", e))?;
    execute!(stdout, EnterAlternateScreen).map_err(|e| format!("alternate screen: {}", e))?;

    // Try to enable keyboard enhancement (key release detection).
    // This silently fails on terminals that don't support it.
    let has_key_release = queue!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .is_ok()
        && stdout.flush().is_ok();

    let mut octave: u8 = 4;

    print_banner(&mut stdout, octave, has_key_release);

    // For terminals without key release: track auto-off timers
    let (off_tx, off_rx) = std_mpsc::channel::<char>();

    let result = event_loop(
        &engine,
        &mut stdout,
        &mut octave,
        has_key_release,
        &off_tx,
        &off_rx,
    );

    // Restore terminal
    let _ = engine.send(LiveCommand::AllNotesOff);
    std::thread::sleep(Duration::from_millis(20));
    let _ = engine.send(LiveCommand::Shutdown);

    if has_key_release {
        let _ = execute!(
            stdout,
            crossterm::event::PopKeyboardEnhancementFlags,
            LeaveAlternateScreen
        );
    } else {
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
    let _ = terminal::disable_raw_mode();

    result
}

fn event_loop(
    engine: &AudioEngine,
    stdout: &mut io::Stdout,
    octave: &mut u8,
    has_key_release: bool,
    off_tx: &std_mpsc::Sender<char>,
    off_rx: &std_mpsc::Receiver<char>,
) -> Result<(), String> {
    loop {
        // Check for any pending auto-off messages (non-blocking)
        while let Ok(key) = off_rx.try_recv() {
            engine.send(LiveCommand::NoteOff { key })?;
        }

        // Poll for keyboard events (50ms timeout so we can also drain off_rx)
        if !event::poll(Duration::from_millis(50))
            .map_err(|e| format!("event poll error: {}", e))?
        {
            continue;
        }

        let ev = event::read().map_err(|e| format!("event read error: {}", e))?;

        match ev {
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                kind: KeyEventKind::Press,
                ..
            }) => {
                return Ok(());
            }

            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                kind: KeyEventKind::Press,
                ..
            }) => {
                // Octave change with number keys
                if let Some(digit) = c.to_digit(10) {
                    if (1..=8).contains(&digit) {
                        *octave = digit as u8;
                        update_status(stdout, *octave, None);
                        continue;
                    }
                }

                // Note key
                if let Some((note_name, oct_offset)) = char_to_note(c) {
                    let effective_octave = octave.saturating_add(oct_offset).min(8);
                    let freq = note_name.to_freq(effective_octave);
                    engine.send(LiveCommand::NoteOn { key: c, freq })?;
                    update_status(stdout, *octave, Some(format!("{:?}{}", note_name, effective_octave)));

                    if !has_key_release {
                        // Schedule auto note-off after 300ms
                        let tx = off_tx.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(300));
                            let _ = tx.send(c);
                        });
                    }
                }
            }

            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                kind: KeyEventKind::Release,
                ..
            }) => {
                if char_to_note(c).is_some() {
                    engine.send(LiveCommand::NoteOff { key: c })?;
                    update_status(stdout, *octave, None);
                }
            }

            _ => {}
        }
    }
}

fn print_banner(stdout: &mut io::Stdout, octave: u8, has_key_release: bool) {
    let release_info = if has_key_release {
        "key release detected - hold keys to sustain"
    } else {
        "no key release support - notes auto-stop after 300ms"
    };

    let banner = format!(
        "\x1b[2J\x1b[H\
clidaw live - interactive keyboard mode\r\n\
─────────────────────────────────────────\r\n\
\r\n\
  Natural notes:  a s d f g h j k l ; '\r\n\
                  C D E F G A B C D E F\r\n\
\r\n\
  Sharps/flats:   w e   t y u   o p\r\n\
                  C# D#  F# G# A#  C# D#\r\n\
\r\n\
  Octave (1-8):   press number keys\r\n\
  Quit:           Esc\r\n\
\r\n\
  {}\r\n\
\r\n",
        release_info
    );
    let _ = write!(stdout, "{}", banner);
    update_status(stdout, octave, None);
}

fn update_status(stdout: &mut io::Stdout, octave: u8, note: Option<String>) {
    let note_display = note.unwrap_or_else(|| "---".to_string());
    // Move to a fixed line for status, clear it, write status
    let _ = write!(
        stdout,
        "\x1b[16;1H\x1b[2K  Octave: {}  |  Note: {}\r",
        octave, note_display
    );
    let _ = stdout.flush();
}
