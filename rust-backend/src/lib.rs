use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::collections::HashMap;
use std::time::Duration;
use once_cell::sync::Lazy;

pub mod audio_sampler;
pub mod input_handler;
pub mod effects;
pub mod installer;
pub mod led_driver;
pub mod lighting;
pub mod presets;
pub mod settings;

use crate::effects::Effect;
use crate::led_driver::{Color, LedController};
use crate::presets::{
    aurora::AuroraEffect,
    breathing::ColorBreathEffect,
    horse::HorseEffect,
    horseCycle::SmoothHorseCycleEffect,
    off::OffEffect,
    pulse::PulseCenterEffect,
    rainbowBreath::RainbowBreathEffect,
    rainbowCycle::RainbowCycleEffect,
    rainbowWave::RainbowWaveEffect,
    rpm::FerrariRpmEffect,
    scan::ColorScanEffect,
    sparkle::SparkleEffect,
    sweep::RgbSweepEffect,
    wheel::ColorWheelEffect,
    thermalStatus::ThermalStatusEffect,
    audio_sparkle::AudioSparkleEffect,
    audio_sparkle_rainbow::AudioSparkleRainbowEffect,
    audio_ripple::AudioRippleEffect,
    rainbow_ripple::RainbowRippleEffect,
    ParameterValue,
    PresetConfig,
};
use crate::presets::{
    ambient::AmbientEffect,
    audio_sparkle_media::AudioSparkleMediaEffect,
};

/// Public entry point for the web-server binary to build an effect without FFI.
/// Returns a boxed Effect or an error string.
pub fn build_effect(
    name: &str,
    parameters: std::collections::HashMap<String, ParameterValue>,
) -> Result<Box<dyn Effect>, String> {
    apply_preset_raw(name.to_string(), parameters)
}

const NUM_ZONES: usize = 24;

pub struct AppState {
    pub controller: Mutex<LedController>,
    pub ui_frame: Arc<Mutex<Vec<Color>>>,
    pub should_run_effect: Arc<AtomicBool>,
    pub current_effect: Mutex<Option<Box<dyn Effect>>>,
    pub current_preset_params: Mutex<HashMap<String, ParameterValue>>,
    pub preset_cycle_index: std::sync::atomic::AtomicUsize,
}

static APP_STATE: Lazy<Mutex<Option<Arc<AppState>>>> = Lazy::new(|| Mutex::new(None));

pub type FrameCallback = unsafe extern "C" fn(*const Color, i32);
static FRAME_CALLBACK: Lazy<Mutex<Option<FrameCallback>>> = Lazy::new(|| Mutex::new(None));

// ===================================================================
// FFI HELPERS
// ===================================================================

unsafe fn c_str_to_str<'a>(ptr: *const c_char) -> Result<&'a str, String> {
    if ptr.is_null() {
        return Err("Null pointer passed for string".to_string());
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map_err(|e| format!("Invalid UTF-8 string: {}", e))
}

fn string_to_c_char(s: String) -> *mut c_char {
    let c_str = CString::new(s).unwrap_or_else(|_| CString::new("Error: CString conversion failed").unwrap());
    c_str.into_raw()
}

// ===================================================================
// CORE API EXPORTS
// ===================================================================

#[no_mangle]
pub extern "C" fn rgb_init() -> i32 {
    let mut state_lock = APP_STATE.lock().unwrap();
    if state_lock.is_none() {
        let ui_frame = Arc::new(Mutex::new(vec![Color::black(); NUM_ZONES]));
        let mut controller = LedController::new(ui_frame.clone());
        
        let settings = settings::load_settings().unwrap_or_default();
        let _ = controller.connect();
        controller.set_brightness(settings.brightness_level);
        let _ = controller.flush_buffered();

        let state = Arc::new(AppState {
            controller: Mutex::new(controller),
            ui_frame: ui_frame.clone(),
            should_run_effect: Arc::new(AtomicBool::new(true)),
            current_effect: Mutex::new(None),
            current_preset_params: Mutex::new(HashMap::new()),
            preset_cycle_index: std::sync::atomic::AtomicUsize::new(0),
        });
        
        *state_lock = Some(state.clone());

        // Start key listener
        input_handler::start_key_listener();

        // Run startup fixes if configured (Windows-only features)
        #[cfg(target_os = "windows")]
        if settings.auto_fix_on_startup {
            if !installer::is_startup_task_installed() {
                let _ = installer::create_startup_task(settings.startup_delay_seconds);
            }
            if settings.fix_on_app_launch {
                let _ = lighting::set_windows_lighting_on_top();
            }
        }

        // Start effect loop in background thread
        let loop_state = state.clone();
        std::thread::spawn(move || {
            let start_time = std::time::Instant::now();
            let mut last_update = std::time::Instant::now();
            let mut consecutive_hid_failures: u32 = 0;
            const MAX_CONSECUTIVE_FAILURES: u32 = 5;
            const SLEEP_DELTA_THRESHOLD: f32 = 2.0; // seconds — anything above this means we just resumed

            loop {
                // If AppState was destroyed, exit loop
                if APP_STATE.lock().unwrap().is_none() {
                    break;
                }

                if !loop_state.should_run_effect.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(16));
                    continue;
                }

                let now = std::time::Instant::now();
                let raw_delta = (now - last_update).as_secs_f32();
                last_update = now;

                // Sleep/resume detection: if the gap is huge, the system just woke up.
                // Clamp delta to one normal frame so effects don't compute with a multi-hour delta.
                let delta = if raw_delta > SLEEP_DELTA_THRESHOLD {
                    // Reconnect the HID device — the old handle is almost certainly stale
                    {
                        let mut controller = loop_state.controller.lock().unwrap();
                        let _ = controller.reconnect();
                    }
                    consecutive_hid_failures = 0;
                    0.04 // one normal frame at 25fps
                } else {
                    raw_delta
                };

                let time = (now - start_time).as_secs_f32();

                // Update current effect
                {
                    let mut current_effect = loop_state.current_effect.lock().unwrap();
                    if let Some(ref mut effect) = *current_effect {
                        let mut controller = loop_state.controller.lock().unwrap();
                        effect.update(&mut controller, time, delta);
                    }
                }

                // Copy buffer and trigger callback
                let frame = {
                    let controller = loop_state.controller.lock().unwrap();
                    let current_frame = controller.get_buffer_vec();
                    *loop_state.ui_frame.lock().unwrap() = current_frame.clone();
                    current_frame
                };

                if let Some(callback) = *FRAME_CALLBACK.lock().unwrap() {
                    unsafe {
                        callback(frame.as_ptr(), frame.len() as i32);
                    }
                }

                // Passive reconnection: if HID writes are silently failing (effects use `let _ = ...`),
                // detect via is_connected() check after the update and attempt recovery.
                {
                    let controller = loop_state.controller.lock().unwrap();
                    if !controller.is_connected() {
                        consecutive_hid_failures += 1;
                    } else {
                        consecutive_hid_failures = 0;
                    }
                }

                if consecutive_hid_failures >= MAX_CONSECUTIVE_FAILURES {
                    let mut controller = loop_state.controller.lock().unwrap();
                    let _ = controller.reconnect();
                    consecutive_hid_failures = 0;
                }

                std::thread::sleep(Duration::from_millis(40));
            }
        });

        1 // Success
    } else {
        0 // Already initialized
    }
}

#[no_mangle]
pub extern "C" fn rgb_shutdown() {
    *FRAME_CALLBACK.lock().unwrap() = None;

    let mut state_lock = APP_STATE.lock().unwrap();
    if let Some(state) = state_lock.take() {
        // Stop active effect
        let mut current_effect = state.current_effect.lock().unwrap();
        if let Some(mut effect) = current_effect.take() {
            let mut controller = state.controller.lock().unwrap();
            effect.stop(&mut controller);
        }
        
        // Turn off LEDs
        let mut controller = state.controller.lock().unwrap();
        controller.fill(Color::black());
        let _ = controller.flush_buffered();
        controller.disconnect();
    }
}

#[no_mangle]
pub extern "C" fn rgb_start_frame_callback(callback: Option<FrameCallback>) {
    *FRAME_CALLBACK.lock().unwrap() = callback;
}

#[no_mangle]
pub extern "C" fn rgb_get_brightness() -> f32 {
    let state_lock = APP_STATE.lock().unwrap();
    if let Some(ref state) = *state_lock {
        state.controller.lock().unwrap().brightness()
    } else {
        1.0
    }
}

#[no_mangle]
pub extern "C" fn rgb_set_brightness(brightness: f32) -> i32 {
    let state_lock = APP_STATE.lock().unwrap();
    let state = match &*state_lock {
        Some(s) => s,
        None => return 0,
    };

    let b = brightness.clamp(0.0, 1.0);

    // Persist brightness
    let mut s = match settings::load_settings() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    s.brightness_level = b;
    if settings::save_settings(&s).is_err() {
        return 0;
    }

    let mut controller = state.controller.lock().unwrap();
    controller.set_brightness(b);

    // Update UI frame immediately
    let scaled_frame: Vec<Color> = controller
        .get_buffer_vec()
        .iter()
        .map(|c| c.perceptual_scale(b))
        .collect();

    *state.ui_frame.lock().unwrap() = scaled_frame.clone();

    if !controller.is_connected() {
        let _ = controller.connect();
    }

    if controller.is_connected() {
        let _ = controller.flush_buffered();
    }

    1
}

#[no_mangle]
pub extern "C" fn rgb_get_frame(buffer: *mut Color, len: i32) -> i32 {
    if buffer.is_null() || len < NUM_ZONES as i32 {
        return 0;
    }

    let state_lock = APP_STATE.lock().unwrap();
    if let Some(ref state) = *state_lock {
        let frame = state.ui_frame.lock().unwrap();
        unsafe {
            std::ptr::copy_nonoverlapping(frame.as_ptr(), buffer, NUM_ZONES);
        }
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn rgb_get_preset_metadata() -> *mut c_char {
    let metadata = presets::get_preset_metadata();
    let json = serde_json::to_string(&metadata).unwrap_or_else(|_| "[]".to_string());
    string_to_c_char(json)
}

#[no_mangle]
pub extern "C" fn rgb_set_preset(preset_name: *const c_char, params_json: *const c_char) -> *mut c_char {
    let name_str = match unsafe { c_str_to_str(preset_name) } {
        Ok(s) => s.to_string(),
        Err(e) => return string_to_c_char(format!("Error: {}", e)),
    };

    let params_str = match unsafe { c_str_to_str(params_json) } {
        Ok(s) => s,
        Err(e) => return string_to_c_char(format!("Error: {}", e)),
    };

    let parameters: HashMap<String, ParameterValue> = match serde_json::from_str(params_str) {
        Ok(p) => p,
        Err(e) => return string_to_c_char(format!("Error deserializing parameters: {}", e)),
    };

    // Save parameter tweaks
    if let Ok(mut settings) = settings::load_settings() {
        if !parameters.is_empty() {
            settings.preset_tweaks.insert(name_str.clone(), parameters.clone());
            let _ = settings::save_settings(&settings);
        }
    }

    let state_lock = APP_STATE.lock().unwrap();
    if let Some(ref state) = *state_lock {
        // Sync the cycle index to match the manually selected preset if it's in the cycle list
        if let Ok(settings) = settings::load_settings() {
            if let Some(pos) = settings.preset_cycle_effects.iter().position(|x| x.eq_ignore_ascii_case(&name_str)) {
                state.preset_cycle_index.store(pos, std::sync::atomic::Ordering::SeqCst);
            }
        }

        match apply_preset(name_str, parameters, state) {
            Ok(msg) => string_to_c_char(msg),
            Err(e) => string_to_c_char(format!("Error: {}", e)),
        }
    } else {
        string_to_c_char("Error: AppState not initialized".to_string())
    }
}

#[no_mangle]
pub extern "C" fn rgb_adjust_parameter(preset_name: *const c_char, param_name: *const c_char, value_json: *const c_char) -> *mut c_char {
    let name_str = match unsafe { c_str_to_str(preset_name) } {
        Ok(s) => s.to_string(),
        Err(e) => return string_to_c_char(format!("Error: {}", e)),
    };

    let param_str = match unsafe { c_str_to_str(param_name) } {
        Ok(s) => s.to_string(),
        Err(e) => return string_to_c_char(format!("Error: {}", e)),
    };

    let val_str = match unsafe { c_str_to_str(value_json) } {
        Ok(s) => s,
        Err(e) => return string_to_c_char(format!("Error: {}", e)),
    };

    let value: ParameterValue = match serde_json::from_str(val_str) {
        Ok(v) => v,
        Err(e) => return string_to_c_char(format!("Error deserializing parameter value: {}", e)),
    };

    let state_lock = APP_STATE.lock().unwrap();
    if let Some(ref state) = *state_lock {
        // Update current preset params
        {
            let mut params = state.current_preset_params.lock().unwrap();
            params.insert(param_str.clone(), value.clone());
        }

        // Update settings tweaks
        if let Ok(mut settings) = settings::load_settings() {
            let tweaks = settings.preset_tweaks.entry(name_str.clone()).or_insert_with(HashMap::new);
            tweaks.insert(param_str, value);
            let _ = settings::save_settings(&settings);
        }

        // Reapply preset
        let current_params = state.current_preset_params.lock().unwrap().clone();
        match apply_preset(name_str, current_params, state) {
            Ok(msg) => string_to_c_char(msg),
            Err(e) => string_to_c_char(format!("Error: {}", e)),
        }
    } else {
        string_to_c_char("Error: AppState not initialized".to_string())
    }
}

#[no_mangle]
pub extern "C" fn rgb_connect_keyboard(vid: u16, pid: u16) -> i32 {
    let state_lock = APP_STATE.lock().unwrap();
    if let Some(ref state) = *state_lock {
        let mut controller = state.controller.lock().unwrap();
        controller.disconnect();
        match controller.connect_device(vid, pid) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn rgb_enable_dynamic_lighting() -> i32 {
    if lighting::enable_windows_lighting().is_ok() { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn rgb_disable_dynamic_lighting() -> i32 {
    if lighting::disable_windows_lighting().is_ok() { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn rgb_is_dynamic_lighting_enabled() -> i32 {
    if lighting::is_windows_lighting_enabled() { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn rgb_set_lighting_priority() -> *mut c_char {
    match lighting::set_windows_lighting_on_top() {
        Ok(_) => string_to_c_char("Windows Dynamic Lighting Controller set to top priority.".to_string()),
        Err(e) => string_to_c_char(format!("Error: {}", e)),
    }
}

#[no_mangle]
pub extern "C" fn rgb_check_startup_installed() -> i32 {
    if installer::is_startup_task_installed() { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn rgb_install_startup_task(delay_seconds: u32) -> *mut c_char {
    match installer::create_startup_task(delay_seconds) {
        Ok(_) => string_to_c_char("Startup task installed successfully.".to_string()),
        Err(e) => string_to_c_char(format!("Error: {}", e)),
    }
}

#[no_mangle]
pub extern "C" fn rgb_uninstall_startup_task() -> *mut c_char {
    match installer::remove_startup_task() {
        Ok(_) => string_to_c_char("Startup task uninstalled successfully.".to_string()),
        Err(e) => string_to_c_char(format!("Error: {}", e)),
    }
}

#[no_mangle]
pub extern "C" fn rgb_get_settings() -> *mut c_char {
    match settings::load_settings() {
        Ok(s) => {
            let json = serde_json::to_string(&s).unwrap_or_else(|_| "{}".to_string());
            string_to_c_char(json)
        }
        Err(e) => string_to_c_char(format!("Error: {}", e)),
    }
}

#[no_mangle]
pub extern "C" fn rgb_save_settings(settings_json: *const c_char) -> *mut c_char {
    let json_str = match unsafe { c_str_to_str(settings_json) } {
        Ok(s) => s,
        Err(e) => return string_to_c_char(format!("Error: {}", e)),
    };

    let settings_obj: settings::AppSettings = match serde_json::from_str(json_str) {
        Ok(s) => s,
        Err(e) => return string_to_c_char(format!("Error deserializing settings: {}", e)),
    };

    match settings::save_settings(&settings_obj) {
        Ok(_) => string_to_c_char("Settings saved successfully.".to_string()),
        Err(e) => string_to_c_char(format!("Error: {}", e)),
    }
}

#[no_mangle]
pub extern "C" fn rgb_cycle_preset() -> *mut c_char {
    let state_lock = APP_STATE.lock().unwrap();
    let state = match &*state_lock {
        Some(s) => s,
        None => return string_to_c_char("Error: AppState not initialized".to_string()),
    };

    let settings = match settings::load_settings() {
        Ok(s) => s,
        Err(e) => return string_to_c_char(format!("Error loading settings: {}", e)),
    };
    
    let effects = settings.preset_cycle_effects;
    if effects.is_empty() {
        return string_to_c_char("".to_string());
    }

    let current_index = state.preset_cycle_index.load(Ordering::SeqCst);
    let next_index = (current_index + 1) % effects.len();
    state.preset_cycle_index.store(next_index, Ordering::SeqCst);

    let next_effect = &effects[next_index];
    
    let parameters = settings
        .preset_tweaks
        .get(next_effect)
        .cloned()
        .unwrap_or_else(HashMap::new);
    
    match apply_preset(next_effect.clone(), parameters, state) {
        Ok(_) => string_to_c_char(next_effect.clone()),
        Err(e) => string_to_c_char(format!("Error: {}", e)),
    }
}

#[no_mangle]
pub extern "C" fn rgb_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// ===================================================================
// EFFECT DISPATCH
// ===================================================================

/// Public: build a boxed Effect from a name+params map, without needing AppState.
/// Used by the web-server binary and by apply_preset internally.
pub fn apply_preset_raw(
    preset_name: String,
    parameters: HashMap<String, ParameterValue>,
) -> Result<Box<dyn Effect>, String> {
    let preset_config = PresetConfig { name: preset_name, parameters };
    let preset_name_lc = preset_config.name.to_lowercase();
    build_effect_inner(&preset_name_lc, &preset_config)
}

fn build_effect_inner(
    name_lc: &str,
    preset_config: &PresetConfig,
) -> Result<Box<dyn Effect>, String> {
    macro_rules! getf {
        ($k:expr, $d:expr) => {
            preset_config.parameters.get($k)
                .and_then(|v| match v { ParameterValue::Float(f) => Some(*f), _ => None })
                .unwrap_or($d)
        };
    }
    macro_rules! getc {
        ($k:expr, $r:expr, $g:expr, $b:expr) => {
            preset_config.parameters.get($k)
                .and_then(|v| match v { ParameterValue::Color { r, g, b } => Some(Color::new(*r, *g, *b)), _ => None })
                .unwrap_or(Color::new($r, $g, $b))
        };
    }
    let e: Box<dyn Effect> = match name_lc {
        "staticcolor"       => {
            let mut colors = [Color::white(); 24];
            for i in 0..24 {
                let key = format!("color{}", i + 1);
                colors[i] = getc!(&key, 255, 255, 255);
            }
            Box::new(crate::presets::staticColor::StaticEffect::new(colors))
        }
        "off"               => Box::new(OffEffect::new()),
        "rainbowcycle"      => Box::new(RainbowCycleEffect::new(getf!("speed", 1.0))),
        "rainbowwave"       => Box::new(RainbowWaveEffect::new(getf!("speed", 1.0))),
        "sweep"             => Box::new(RgbSweepEffect::new()),
        "rainbowbreath"     => Box::new(RainbowBreathEffect::new(getf!("speed", 1.0))),
        "thermalstatus"     => Box::new(ThermalStatusEffect::new()),
        "breathing"         => Box::new(ColorBreathEffect::new(getc!("color", 255, 0, 0), getf!("speed", 1.0))),
        "horse"             => Box::new(HorseEffect::new(getf!("speed", 1.0), getf!("length", 3.0), getc!("base_color", 20, 20, 25), getc!("horse_color", 120, 140, 180))),
        "horsecycle"        => Box::new(SmoothHorseCycleEffect::new(getf!("speed", 1.0), getf!("length", 3.0))),
        "rpm"               => Box::new(FerrariRpmEffect::new()),
        "pulse"             => Box::new(PulseCenterEffect::new(getc!("color", 255, 0, 0), getf!("speed", 1.0))),
        "wheel"             => Box::new(ColorWheelEffect::new(getf!("speed", 0.5))),
        "aurora"            => Box::new(AuroraEffect::new(getf!("speed", 0.5))),
        "scan"              => Box::new(ColorScanEffect::new(getf!("speed", 1.0))),
        "sparkle"           => Box::new(SparkleEffect::new(getf!("density", 0.1))),
        "nebula"            => Box::new(crate::presets::nebula::NebulaEffect::new(getf!("speed", 1.0))),
        "chromaticbreath"   => Box::new(crate::presets::chromaticBreath::ChromaticBreathEffect::new(getf!("speed", 1.0))),
        "audio_sparkle" => {
            let sampler = crate::audio_sampler::AudioSampler::new(getf!("audio_source", 0.0)).map_err(|e| e.to_string())?;
            Box::new(AudioSparkleEffect::new(sampler, getf!("sensitivity", 1.0), getf!("base_density", 0.05)))
        }
        "audio_sparkle_rainbow" => {
            let sampler = crate::audio_sampler::AudioSampler::new(getf!("audio_source", 0.0)).map_err(|e| e.to_string())?;
            Box::new(AudioSparkleRainbowEffect::new(sampler, getf!("sensitivity", 1.0), getf!("base_density", 0.0), getf!("rainbow_speed", 1.0)))
        }
        "audio_ripple" => {
            let sampler = crate::audio_sampler::AudioSampler::new(getf!("audio_source", 0.0)).map_err(|e| e.to_string())?;
            Box::new(AudioRippleEffect::new(sampler, getf!("sensitivity", 1.0), getf!("speed", 40.0), getf!("width", 3.0), getf!("lifetime", 0.8)))
        }
        "rainbow_ripple"    => Box::new(RainbowRippleEffect::new(getf!("speed", 40.0), getf!("width", 3.0), getf!("lifetime", 0.8))),
        "ambient" => {
            let smoothing = getf!("smoothing", 1.0);
            let mut sampler = crate::presets::ambient::DxgiScreenSampler::new().map_err(|e| e.to_string())?;
            let l = preset_config.parameters.get("sample_left").and_then(|v| match v { ParameterValue::Float(f) => Some(*f), _ => None });
            let w = preset_config.parameters.get("sample_width").and_then(|v| match v { ParameterValue::Float(f) => Some(*f), _ => None });
            if let (Some(l), Some(w)) = (l, w) { sampler.set_sample_horizontal_region(l, w); }
            else if let Ok(s) = crate::settings::load_settings() { sampler.set_sample_horizontal_region(s.ambient_sample_left_fraction, s.ambient_sample_width_fraction); }
            Box::new(AmbientEffect::new(sampler, smoothing))
        }
        "audio_sparkle_media" => {
            let sampler_audio = crate::audio_sampler::AudioSampler::new(getf!("audio_source", 0.0)).map_err(|e| e.to_string())?;
            let mut sampler_media = crate::presets::ambient::DxgiScreenSampler::new().map_err(|e| e.to_string())?;
            sampler_media.set_sample_top_fraction(0.15);
            sampler_media.set_sample_horizontal_region(0.0, 1.0);
            Box::new(AudioSparkleMediaEffect::new(sampler_audio, sampler_media, getf!("sensitivity", 1.0), getf!("base_density", 0.0)))
        }
        "layered" => {
            #[derive(serde::Deserialize)]
            struct LayerConfig {
                name: String,
                opacity: f32,
                priority: i32,
                parameters: HashMap<String, serde_json::Value>,
            }
            let config_str = match preset_config.parameters.get("config") {
                Some(ParameterValue::String(s)) => s,
                _ => return Err("Missing or invalid 'config' parameter for layered effect".to_string()),
            };
            let layers: Vec<LayerConfig> = serde_json::from_str(config_str)
                .map_err(|e| format!("Invalid layered config JSON: {}", e))?;
            
            let mut effect_layers = Vec::new();
            for layer in layers {
                let name_lc = layer.name.to_lowercase();
                let mut converted_params = HashMap::new();
                for (k, v) in layer.parameters {
                    let pv = ParameterValue::from_json(&v)
                        .ok_or_else(|| format!("Invalid value for param '{}' in layer '{}'", k, layer.name))?;
                    converted_params.insert(k, pv);
                }
                let sub_preset_config = PresetConfig {
                    name: layer.name,
                    parameters: converted_params,
                };
                let sub_effect = build_effect_inner(&name_lc, &sub_preset_config)?;
                effect_layers.push(crate::presets::layered::EffectLayer {
                    effect: sub_effect,
                    opacity: layer.opacity,
                    priority: layer.priority,
                });
            }
            Box::new(crate::presets::layered::LayeredEffect::new(effect_layers))
        }
        _ => return Err(format!("Unknown preset: {}", preset_config.name)),
    };
    Ok(e)
}

fn apply_preset(
    preset_name: String,
    parameters: HashMap<String, ParameterValue>,
    state: &AppState,
) -> Result<String, String> {
    // Stop current effect
    {
        let mut current_effect = state.current_effect.lock().unwrap();
        if let Some(mut effect) = current_effect.take() {
            let mut controller = state.controller.lock().unwrap();
            effect.stop(&mut controller);
        }
    }

    // Store parameters
    {
        let mut params = state.current_preset_params.lock().unwrap();
        params.clear();
        for (key, value) in &parameters {
            params.insert(key.clone(), value.clone());
        }
    }

    let name_copy = preset_name.clone();
    let new_effect = apply_preset_raw(preset_name, parameters)?;

    {
        let mut current_effect = state.current_effect.lock().unwrap();
        *current_effect = Some(new_effect);
    }

    Ok(format!("Preset '{}' loaded successfully", name_copy))
}

