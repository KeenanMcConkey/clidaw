use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

    // Enable keyboard enhancement for key release and repeat detection.
    // We always try to enable it, and use a hybrid approach:
    // - If Release events work, great!
    // - If only Repeat events work, we use those to detect held keys
    // - If neither work reliably, we fall back to timeout-based release
    let kb_enhanced = queue!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .is_ok()
        && stdout.flush().is_ok();
    
    // On macOS, even if enhancement succeeds, Release events may not work
    // so we always use the fallback logic there
    let has_key_release = kb_enhanced && !cfg!(target_os = "macos");

    let mut octave: u8 = 4;

    print_banner(&mut stdout, octave);

    let result = event_loop(&engine, &mut stdout, &mut octave, has_key_release);

    // Restore terminal
    let _ = engine.send(LiveCommand::AllNotesOff);
    std::thread::sleep(Duration::from_millis(20));
    let _ = engine.send(LiveCommand::Shutdown);

    if kb_enhanced {
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
) -> Result<(), String> {
    // For the fallback path: track when each key was last pressed/repeated
    // so we can detect when a key is released (no more repeat events)
    let active_keys: Arc<Mutex<HashMap<char, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // Channel to receive keys that should be released
    let (release_tx, release_rx) = std_mpsc::channel::<char>();
    
    // Channel to signal the monitor thread to shut down
    let (shutdown_tx, shutdown_rx) = std_mpsc::channel::<()>();

    // Spawn a background thread that checks for keys that haven't been updated recently
    if !has_key_release {
        let keys_clone = Arc::clone(&active_keys);
        let tx_clone = release_tx.clone();
        std::thread::spawn(move || {
            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }
                
                std::thread::sleep(Duration::from_millis(50));
                let now = Instant::now();
                let mut keys = keys_clone.lock().unwrap();
                let mut to_release = Vec::new();
                
                // Find keys that haven't been updated in the last 100ms
                // (meaning no repeat events, so the key was released)
                for (key, last_time) in keys.iter() {
                    if now.duration_since(*last_time) > Duration::from_millis(100) {
                        to_release.push(*key);
                    }
                }
                
                // Remove and send release events for stale keys
                for key in to_release {
                    keys.remove(&key);
                    let _ = tx_clone.send(key);
                }
            }
        });
    }

    loop {
        // Drain any release messages from the monitor thread
        if !has_key_release {
            while let Ok(key) = release_rx.try_recv() {
                engine.send(LiveCommand::NoteOff { key })?;
                update_status(stdout, *octave, None);
            }
        }

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
                // Signal the monitor thread to shut down
                let _ = shutdown_tx.send(());
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

                    // Track this key as active for the fallback path
                    if !has_key_release {
                        let mut keys = active_keys.lock().unwrap();
                        keys.insert(c, Instant::now());
                    }
                }
            }

            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                kind: KeyEventKind::Repeat,
                ..
            }) => {
                // Key is being held - update its timestamp so it doesn't get released
                if !has_key_release && char_to_note(c).is_some() {
                    let mut keys = active_keys.lock().unwrap();
                    keys.insert(c, Instant::now());
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

fn print_banner(stdout: &mut io::Stdout, octave: u8) {
    let banner = "\x1b[2J\x1b[H\
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
\r\n";
    let _ = write!(stdout, "{}", banner);
    update_status(stdout, octave, None);
}

fn update_status(stdout: &mut io::Stdout, octave: u8, note: Option<String>) {
    let note_display = note.unwrap_or_else(|| "---".to_string());
    let _ = write!(
        stdout,
        "\x1b[16;1H\x1b[2K  Octave: {}  |  Note: {}\r",
        octave, note_display
    );
    let _ = stdout.flush();
}
