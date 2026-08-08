use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::settings;

/// Queue of recently pressed keyboard zone indices (0-23).
/// Effects like rainbow_ripple consume this to trigger visuals per-keypress.
pub static KEY_EVENTS: Lazy<Mutex<Vec<u32>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub type CycleCallback = Arc<dyn Fn() + Send + Sync + 'static>;
static CYCLE_CALLBACK: Lazy<Mutex<Option<CycleCallback>>> = Lazy::new(|| Mutex::new(None));

pub fn set_cycle_callback<F>(cb: F)
where
    F: Fn() + Send + Sync + 'static,
{
    if let Ok(mut lock) = CYCLE_CALLBACK.lock() {
        *lock = Some(Arc::new(cb));
    }
}

pub fn trigger_cycle_callback() {
    if let Ok(lock) = CYCLE_CALLBACK.lock() {
        if let Some(ref cb) = *lock {
            cb();
        }
    }
}

// ─── Shortcut Parser ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct ParsedShortcut {
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
    key: String,
}

fn parse_shortcut(s: &str) -> Option<ParsedShortcut> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut alt = false;
    let mut ctrl = false;
    let mut shift = false;
    let mut meta = false;
    let mut key = String::new();

    for part in parts {
        match part.to_lowercase().as_str() {
            "alt" => alt = true,
            "ctrl" | "control" => ctrl = true,
            "shift" => shift = true,
            "meta" | "super" | "win" | "cmd" => meta = true,
            k => key = k.to_string(),
        }
    }

    if key.is_empty() {
        return None;
    }

    Some(ParsedShortcut { alt, ctrl, shift, meta, key })
}

// Modifier state trackers
static ALT_HELD: AtomicBool = AtomicBool::new(false);
static CTRL_HELD: AtomicBool = AtomicBool::new(false);
static SHIFT_HELD: AtomicBool = AtomicBool::new(false);
static META_HELD: AtomicBool = AtomicBool::new(false);

// ─── Windows implementation ──────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub fn start_key_listener() {
    use rdev::{listen, Event, EventType, Key};

    std::thread::spawn(|| {
        let callback = |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    // Update modifiers
                    match key {
                        Key::Alt | Key::AltGr => ALT_HELD.store(true, Ordering::Relaxed),
                        Key::ControlLeft | Key::ControlRight => CTRL_HELD.store(true, Ordering::Relaxed),
                        Key::ShiftLeft | Key::ShiftRight => SHIFT_HELD.store(true, Ordering::Relaxed),
                        Key::MetaLeft | Key::MetaRight => META_HELD.store(true, Ordering::Relaxed),
                        _ => {}
                    }

                    // Check shortcut
                    if let Ok(cfg) = settings::load_settings() {
                        if let Some(ref shortcut_str) = cfg.preset_cycle_shortcut {
                            if let Some(target) = parse_shortcut(shortcut_str) {
                                if let Some(key_name) = map_rdev_key_name(key) {
                                    if target.key.eq_ignore_ascii_case(&key_name)
                                        && target.alt == ALT_HELD.load(Ordering::Relaxed)
                                        && target.ctrl == CTRL_HELD.load(Ordering::Relaxed)
                                        && target.shift == SHIFT_HELD.load(Ordering::Relaxed)
                                        && target.meta == META_HELD.load(Ordering::Relaxed)
                                    {
                                        trigger_cycle_callback();
                                    }
                                }
                            }
                        }
                    }

                    let zone = map_key_to_zone_win(key);
                    if let Ok(mut events) = KEY_EVENTS.lock() {
                        events.push(zone);
                        if events.len() > 10 {
                            events.remove(0);
                        }
                    }
                }
                EventType::KeyRelease(key) => {
                    match key {
                        Key::Alt | Key::AltGr => ALT_HELD.store(false, Ordering::Relaxed),
                        Key::ControlLeft | Key::ControlRight => CTRL_HELD.store(false, Ordering::Relaxed),
                        Key::ShiftLeft | Key::ShiftRight => SHIFT_HELD.store(false, Ordering::Relaxed),
                        Key::MetaLeft | Key::MetaRight => META_HELD.store(false, Ordering::Relaxed),
                        _ => {}
                    }
                }
                _ => {}
            }
        };
        if let Err(error) = listen(callback) {
            eprintln!("[input_handler] Key listener error: {:?}", error);
        }
    });
}

#[cfg(target_os = "windows")]
fn map_rdev_key_name(key: rdev::Key) -> Option<String> {
    use rdev::Key;
    let s = match key {
        Key::KeyA => "a", Key::KeyB => "b", Key::KeyC => "c", Key::KeyD => "d",
        Key::KeyE => "e", Key::KeyF => "f", Key::KeyG => "g", Key::KeyH => "h",
        Key::KeyI => "i", Key::KeyJ => "j", Key::KeyK => "k", Key::KeyL => "l",
        Key::KeyM => "m", Key::KeyN => "n", Key::KeyO => "o", Key::KeyP => "p",
        Key::KeyQ => "q", Key::KeyR => "r", Key::KeyS => "s", Key::KeyT => "t",
        Key::KeyU => "u", Key::KeyV => "v", Key::KeyW => "w", Key::KeyX => "x",
        Key::KeyY => "y", Key::KeyZ => "z",
        Key::Num0 => "0", Key::Num1 => "1", Key::Num2 => "2", Key::Num3 => "3",
        Key::Num4 => "4", Key::Num5 => "5", Key::Num6 => "6", Key::Num7 => "7",
        Key::Num8 => "8", Key::Num9 => "9",
        Key::F1 => "f1", Key::F2 => "f2", Key::F3 => "f3", Key::F4 => "f4",
        Key::F5 => "f5", Key::F6 => "f6", Key::F7 => "f7", Key::F8 => "f8",
        Key::F9 => "f9", Key::F10 => "f10", Key::F11 => "f11", Key::F12 => "f12",
        Key::Space => "space",
        _ => return None,
    };
    Some(s.to_string())
}

#[cfg(target_os = "windows")]
fn map_key_to_zone_win(key: rdev::Key) -> u32 {
    use rdev::Key;
    match key {
        Key::Escape | Key::BackQuote | Key::Tab | Key::CapsLock | Key::ShiftLeft | Key::ControlLeft => 0,
        Key::F1 | Key::Num1 => 1,
        Key::F2 | Key::Num2 | Key::KeyQ | Key::KeyA => 2,
        Key::KeyW | Key::KeyZ | Key::MetaLeft => 3,
        Key::F3 | Key::Num3 | Key::KeyS | Key::KeyX | Key::Alt => 4,
        Key::F4 | Key::Num4 | Key::KeyE | Key::KeyD => 5,
        Key::F5 | Key::Num5 | Key::KeyR | Key::KeyF | Key::KeyC => 6,
        Key::F6 | Key::KeyT | Key::KeyV => 7,
        Key::F7 | Key::Num6 | Key::KeyG | Key::KeyB => 8,
        Key::Num7 | Key::KeyY | Key::KeyH => 9,
        Key::F8 | Key::KeyU | Key::KeyN => 10,
        Key::F9 | Key::Num8 | Key::KeyJ | Key::KeyM => 11,
        Key::F10 | Key::Num9 | Key::KeyI | Key::KeyK | Key::Comma | Key::AltGr => 12,
        Key::F11 | Key::Num0 | Key::KeyO | Key::KeyL => 13,
        Key::F12 | Key::KeyP | Key::Dot | Key::MetaRight | Key::Function => 14,
        Key::Minus | Key::SemiColon | Key::Slash | Key::LeftArrow => 15,
        Key::Insert | Key::Equal | Key::LeftBracket | Key::Quote | Key::UpArrow | Key::DownArrow => 16,
        Key::PrintScreen | Key::Backspace | Key::RightBracket | Key::Return | Key::ShiftRight => 17,
        Key::Delete | Key::BackSlash | Key::RightArrow => 18,
        Key::Home | Key::NumLock | Key::Kp7 | Key::Kp4 | Key::Kp1 | Key::Kp0 => 20,
        Key::End | Key::KpDivide | Key::Kp8 | Key::Kp5 | Key::Kp2 => 21,
        Key::PageUp | Key::KpMultiply | Key::Kp9 | Key::Kp6 | Key::Kp3 | Key::KpDelete => 22,
        Key::PageDown | Key::KpMinus | Key::KpPlus | Key::KpReturn => 23,
        Key::Space => 8,
        _ => 12,
    }
}

// ─── Linux implementation ────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
pub fn start_key_listener() {
    std::thread::spawn(|| {
        let mut keyboard_paths = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/dev/input") {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        if filename.starts_with("event") {
                            if let Ok(device) = evdev::Device::open(&path) {
                                if let Some(keys) = device.supported_keys() {
                                    if keys.contains(evdev::Key::KEY_A) {
                                        keyboard_paths.push(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        eprintln!("[input_handler] Found {} keyboard devices to monitor.", keyboard_paths.len());

        for path in keyboard_paths {
            std::thread::spawn(move || {
                if let Ok(mut device) = evdev::Device::open(&path) {
                    loop {
                        match device.fetch_events() {
                            Ok(events) => {
                                for event in events {
                                    if let evdev::InputEventKind::Key(key) = event.kind() {
                                        let val = event.value();
                                        
                                        // Track modifier state
                                        match key {
                                            evdev::Key::KEY_LEFTALT | evdev::Key::KEY_RIGHTALT => {
                                                ALT_HELD.store(val != 0, Ordering::Relaxed);
                                            }
                                            evdev::Key::KEY_LEFTCTRL | evdev::Key::KEY_RIGHTCTRL => {
                                                CTRL_HELD.store(val != 0, Ordering::Relaxed);
                                            }
                                            evdev::Key::KEY_LEFTSHIFT | evdev::Key::KEY_RIGHTSHIFT => {
                                                SHIFT_HELD.store(val != 0, Ordering::Relaxed);
                                            }
                                            evdev::Key::KEY_LEFTMETA | evdev::Key::KEY_RIGHTMETA => {
                                                META_HELD.store(val != 0, Ordering::Relaxed);
                                            }
                                            _ => {}
                                        }

                                        // Trigger on press (value == 1)
                                        if val == 1 {
                                            // Check shortcut
                                            if let Ok(cfg) = settings::load_settings() {
                                                if let Some(ref shortcut_str) = cfg.preset_cycle_shortcut {
                                                    if let Some(target) = parse_shortcut(shortcut_str) {
                                                        if let Some(key_name) = map_evdev_key_name(key) {
                                                            if target.key.eq_ignore_ascii_case(key_name)
                                                                && target.alt == ALT_HELD.load(Ordering::Relaxed)
                                                                && target.ctrl == CTRL_HELD.load(Ordering::Relaxed)
                                                                && target.shift == SHIFT_HELD.load(Ordering::Relaxed)
                                                                && target.meta == META_HELD.load(Ordering::Relaxed)
                                                            {
                                                                trigger_cycle_callback();
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            let zone = map_key_to_zone_linux(key);
                                            if let Ok(mut events) = KEY_EVENTS.lock() {
                                                events.push(zone);
                                                if events.len() > 10 {
                                                    events.remove(0);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                }
            });
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn map_evdev_key_name(key: evdev::Key) -> Option<&'static str> {
    use evdev::Key;
    match key {
        Key::KEY_A => Some("a"), Key::KEY_B => Some("b"), Key::KEY_C => Some("c"), Key::KEY_D => Some("d"),
        Key::KEY_E => Some("e"), Key::KEY_F => Some("f"), Key::KEY_G => Some("g"), Key::KEY_H => Some("h"),
        Key::KEY_I => Some("i"), Key::KEY_J => Some("j"), Key::KEY_K => Some("k"), Key::KEY_L => Some("l"),
        Key::KEY_M => Some("m"), Key::KEY_N => Some("n"), Key::KEY_O => Some("o"), Key::KEY_P => Some("p"),
        Key::KEY_Q => Some("q"), Key::KEY_R => Some("r"), Key::KEY_S => Some("s"), Key::KEY_T => Some("t"),
        Key::KEY_U => Some("u"), Key::KEY_V => Some("v"), Key::KEY_W => Some("w"), Key::KEY_X => Some("x"),
        Key::KEY_Y => Some("y"), Key::KEY_Z => Some("z"),
        Key::KEY_0 => Some("0"), Key::KEY_1 => Some("1"), Key::KEY_2 => Some("2"), Key::KEY_3 => Some("3"),
        Key::KEY_4 => Some("4"), Key::KEY_5 => Some("5"), Key::KEY_6 => Some("6"), Key::KEY_7 => Some("7"),
        Key::KEY_8 => Some("8"), Key::KEY_9 => Some("9"),
        Key::KEY_F1 => Some("f1"), Key::KEY_F2 => Some("f2"), Key::KEY_F3 => Some("f3"), Key::KEY_F4 => Some("f4"),
        Key::KEY_F5 => Some("f5"), Key::KEY_F6 => Some("f6"), Key::KEY_F7 => Some("f7"), Key::KEY_F8 => Some("f8"),
        Key::KEY_F9 => Some("f9"), Key::KEY_F10 => Some("f10"), Key::KEY_F11 => Some("f11"), Key::KEY_F12 => Some("f12"),
        Key::KEY_SPACE => Some("space"),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn map_key_to_zone_linux(key: evdev::Key) -> u32 {
    use evdev::Key;
    match key {
        Key::KEY_ESC | Key::KEY_GRAVE | Key::KEY_TAB | Key::KEY_CAPSLOCK | Key::KEY_LEFTSHIFT | Key::KEY_LEFTCTRL => 0,
        Key::KEY_F1 | Key::KEY_1 => 1,
        Key::KEY_F2 | Key::KEY_2 | Key::KEY_Q | Key::KEY_A => 2,
        Key::KEY_W | Key::KEY_Z | Key::KEY_LEFTMETA => 3,
        Key::KEY_F3 | Key::KEY_3 | Key::KEY_S | Key::KEY_X | Key::KEY_LEFTALT => 4,
        Key::KEY_F4 | Key::KEY_4 | Key::KEY_E | Key::KEY_D => 5,
        Key::KEY_F5 | Key::KEY_5 | Key::KEY_R | Key::KEY_F | Key::KEY_C => 6,
        Key::KEY_F6 | Key::KEY_T | Key::KEY_V => 7,
        Key::KEY_F7 | Key::KEY_6 | Key::KEY_G | Key::KEY_B => 8,
        Key::KEY_7 | Key::KEY_Y | Key::KEY_H => 9,
        Key::KEY_F8 | Key::KEY_U | Key::KEY_N => 10,
        Key::KEY_F9 | Key::KEY_8 | Key::KEY_J | Key::KEY_M => 11,
        Key::KEY_F10 | Key::KEY_9 | Key::KEY_I | Key::KEY_K | Key::KEY_COMMA | Key::KEY_RIGHTALT => 12,
        Key::KEY_F11 | Key::KEY_0 | Key::KEY_O | Key::KEY_L => 13,
        Key::KEY_F12 | Key::KEY_P | Key::KEY_DOT | Key::KEY_RIGHTMETA => 14,
        Key::KEY_MINUS | Key::KEY_SEMICOLON | Key::KEY_SLASH | Key::KEY_LEFT => 15,
        Key::KEY_INSERT | Key::KEY_EQUAL | Key::KEY_LEFTBRACE | Key::KEY_APOSTROPHE | Key::KEY_UP | Key::KEY_DOWN => 16,
        Key::KEY_SYSRQ | Key::KEY_BACKSPACE | Key::KEY_RIGHTBRACE | Key::KEY_ENTER | Key::KEY_RIGHTSHIFT => 17,
        Key::KEY_DELETE | Key::KEY_BACKSLASH | Key::KEY_RIGHT => 18,
        Key::KEY_HOME | Key::KEY_NUMLOCK | Key::KEY_KP7 | Key::KEY_KP4 | Key::KEY_KP1 | Key::KEY_KP0 => 20,
        Key::KEY_END | Key::KEY_KPSLASH | Key::KEY_KP8 | Key::KEY_KP5 | Key::KEY_KP2 => 21,
        Key::KEY_PAGEUP | Key::KEY_KPASTERISK | Key::KEY_KP9 | Key::KEY_KP6 | Key::KEY_KP3 | Key::KEY_KPDOT => 22,
        Key::KEY_PAGEDOWN | Key::KEY_KPMINUS | Key::KEY_KPPLUS | Key::KEY_KPENTER => 23,
        Key::KEY_SPACE => 8,
        _ => 12,
    }
}
