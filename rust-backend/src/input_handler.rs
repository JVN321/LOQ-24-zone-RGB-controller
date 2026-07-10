use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Queue of recently pressed keyboard zone indices (0-23).
/// Effects like rainbow_ripple consume this to trigger visuals per-keypress.
pub static KEY_EVENTS: Lazy<Mutex<Vec<u32>>> = Lazy::new(|| Mutex::new(Vec::new()));

// ─── Windows implementation (rdev + X11) ─────────────────────────────────────

#[cfg(target_os = "windows")]
pub fn start_key_listener() {
    use rdev::{listen, Event, EventType};

    std::thread::spawn(|| {
        let callback = |event: Event| {
            if let EventType::KeyPress(key) = event.event_type {
                let zone = map_key_to_zone_win(key);
                if let Ok(mut events) = KEY_EVENTS.lock() {
                    events.push(zone);
                    if events.len() > 10 {
                        events.remove(0);
                    }
                }
            }
        };
        if let Err(error) = listen(callback) {
            eprintln!("[input_handler] Key listener error: {:?}", error);
        }
    });
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
                                    // A general keyboard device supports KEY_A
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
                                        // 1 = Press, 2 = Repeat (let's trigger on press: value == 1)
                                        if event.value() == 1 {
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
                                // Device disconnected or closed
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
