use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

// ─── Re-exports from the backend library ──────────────────────────────────────

use rgb_backend::{
    effects::Effect,
    led_driver::{Color, LedController},
    presets::{self, ParameterValue},
    settings,
};

// ─── Global app state (mirrors the backend AppState, but owned by the server) ─

struct ControllerState {
    controller: LedController,
    ui_frame: Arc<Mutex<Vec<Color>>>,
    current_effect: Option<Box<dyn Effect>>,
    current_params: HashMap<String, ParameterValue>,
    current_preset: String,
    brightness: f32,
    start_time: std::time::Instant,
    last_update: std::time::Instant,
}

type SharedState = Arc<Mutex<ControllerState>>;

// Broadcast channel for live frame pushes to WebSocket clients
type FrameSender = broadcast::Sender<Vec<Color>>;

#[derive(Clone)]
struct AppState {
    ctrl: SharedState,
    frame_tx: FrameSender,
}

// ─── REST API Types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct StatusResponse {
    connected: bool,
    preset: String,
    brightness: f32,
    preset_tweaks: std::collections::HashMap<
        String,
        std::collections::HashMap<String, rgb_backend::presets::ParameterValue>,
    >,
}

#[derive(Deserialize)]
struct SetPresetRequest {
    preset: String,
    #[serde(default)]
    params: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct SetBrightnessRequest {
    brightness: f32,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

fn api_err(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (StatusCode::BAD_REQUEST, Json(ApiError { error: msg.into() }))
}

// ─── Effect loop ──────────────────────────────────────────────────────────────

fn start_effect_loop(state: SharedState, tx: FrameSender) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(40)); // ~25 fps

            let mut s = state.lock().unwrap();
            let now = std::time::Instant::now();
            let raw_delta = (now - s.last_update).as_secs_f32();
            s.last_update = now;
            let delta = raw_delta.min(0.1); // clamp to avoid huge jumps on resume
            let time = (now - s.start_time).as_secs_f32();

            let ControllerState {
                ref mut current_effect,
                ref mut controller,
                ..
            } = *s;

            if let Some(ref mut effect) = current_effect {
                effect.update(controller, time, delta);
            }

            let frame = s.controller.get_buffer_vec();
            *s.ui_frame.lock().unwrap() = frame.clone();

            // Broadcast to all WS clients (ignore send error if no subscribers)
            let _ = tx.send(frame);
        }
    });
}

// ─── Routes ───────────────────────────────────────────────────────────────────

/// GET / — serve the embedded HTML UI
async fn serve_ui() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

/// GET /api/status
async fn get_status(State(app): State<AppState>) -> impl IntoResponse {
    let s = app.ctrl.lock().unwrap();
    let tweaks = if let Ok(cfg) = settings::load_settings() {
        cfg.preset_tweaks
    } else {
        std::collections::HashMap::new()
    };

    Json(StatusResponse {
        connected: s.controller.is_connected(),
        preset: s.current_preset.clone(),
        brightness: s.brightness,
        preset_tweaks: tweaks,
    })
}

/// GET /api/presets — list all available presets with metadata
async fn get_presets() -> impl IntoResponse {
    Json(presets::get_preset_metadata())
}

/// POST /api/preset — apply a preset
async fn set_preset(
    State(app): State<AppState>,
    Json(req): Json<SetPresetRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError>)> {
    // Convert serde_json::Value params → ParameterValue
    let mut params: HashMap<String, ParameterValue> = HashMap::new();
    for (k, v) in &req.params {
        let pv = json_to_param_value(v)
            .ok_or_else(|| api_err(format!("Invalid value for param '{}'", k)))?;
        params.insert(k.clone(), pv);
    }

    let mut s = app.ctrl.lock().unwrap();

    // Stop current effect
    if let Some(mut old) = s.current_effect.take() {
        old.stop(&mut s.controller);
    }
    s.current_params.clear();
    s.current_params.extend(params.clone());
    s.current_preset = req.preset.clone();

    // Build the new effect
    let mut effect = rgb_backend::build_effect(&req.preset, params)
        .map_err(|e| api_err(e))?;

    effect.start();

    s.current_effect = Some(effect);

    // Persist
    if let Ok(mut cfg) = settings::load_settings() {
        cfg.preset_tweaks
            .insert(req.preset.clone(), s.current_params.clone());
        cfg.current_preset = req.preset.clone();
        let _ = settings::save_settings(&cfg);
    }

    Ok(Json(serde_json::json!({ "ok": true, "preset": req.preset })))
}

/// POST /api/brightness
async fn set_brightness(
    State(app): State<AppState>,
    Json(req): Json<SetBrightnessRequest>,
) -> impl IntoResponse {
    let b = req.brightness.clamp(0.0, 1.0);
    let mut s = app.ctrl.lock().unwrap();
    s.brightness = b;
    s.controller.set_brightness(b);

    if let Ok(mut cfg) = settings::load_settings() {
        cfg.brightness_level = b;
        let _ = settings::save_settings(&cfg);
    }

    Json(serde_json::json!({ "ok": true, "brightness": b }))
}

/// GET /api/frame — current 24-zone color frame as JSON
async fn get_frame(State(app): State<AppState>) -> impl IntoResponse {
    let s = app.ctrl.lock().unwrap();
    let frame: Vec<[u8; 3]> = s
        .ui_frame
        .lock()
        .unwrap()
        .iter()
        .map(|c| [c.r, c.g, c.b])
        .collect();
    Json(frame)
}

/// GET /ws — WebSocket for live frame streaming
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| ws_loop(socket, app.frame_tx.subscribe()))
}

async fn ws_loop(mut socket: WebSocket, mut rx: broadcast::Receiver<Vec<Color>>) {
    loop {
        match rx.recv().await {
            Ok(frame) => {
                let payload: Vec<[u8; 3]> =
                    frame.iter().map(|c| [c.r, c.g, c.b]).collect();
                let json = serde_json::to_string(&payload).unwrap_or_default();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => break,
        }
    }
}

// ─── Helper: convert serde_json::Value → ParameterValue ──────────────────────

fn json_to_param_value(v: &serde_json::Value) -> Option<ParameterValue> {
    match v {
        serde_json::Value::Number(n) => Some(ParameterValue::Float(n.as_f64()? as f32)),
        serde_json::Value::String(s) => Some(ParameterValue::String(s.clone())),
        serde_json::Value::Object(map) => {
            let r = map.get("r")?.as_u64()? as u8;
            let g = map.get("g")?.as_u64()? as u8;
            let b = map.get("b")?.as_u64()? as u8;
            Some(ParameterValue::Color { r, g, b })
        }
        _ => None,
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("🎹  LOQ RGB Controller — Web Server");

    // Load settings
    let cfg = settings::load_settings().unwrap_or_default();

    // Build controller
    let ui_frame = Arc::new(Mutex::new(vec![Color::black(); 24]));
    let mut controller = LedController::new(ui_frame.clone());

    match controller.connect() {
        Ok(()) => println!("✅  HID device connected (VID 048d / PID c693)"),
        Err(e) => {
            eprintln!("⚠️  HID device not found: {}", e);
            eprintln!("    Check udev rules: /etc/udev/rules.d/99-loq-rgb.rules");
            eprintln!("    Continuing — effects will run but won't reach hardware.");
        }
    }

    controller.set_brightness(cfg.brightness_level);

    let last_preset = cfg.current_preset.clone();
    let last_tweaks = cfg.preset_tweaks.get(&last_preset).cloned().unwrap_or_default();

    let initial_effect = if !last_preset.is_empty() {
        match rgb_backend::build_effect(&last_preset, last_tweaks.clone()) {
            Ok(mut eff) => {
                eff.start();
                Some(eff)
            }
            Err(e) => {
                eprintln!("⚠️  Failed to restore startup preset '{}': {}", last_preset, e);
                None
            }
        }
    } else {
        None
    };

    let ctrl_state = Arc::new(Mutex::new(ControllerState {
        controller,
        ui_frame: ui_frame.clone(),
        current_effect: initial_effect,
        current_params: last_tweaks,
        current_preset: last_preset,
        brightness: cfg.brightness_level,
        start_time: std::time::Instant::now(),
        last_update: std::time::Instant::now(),
    }));

    let (frame_tx, _) = broadcast::channel::<Vec<Color>>(4);

    // Start key listener (for typing-reactive effects)
    rgb_backend::input_handler::start_key_listener();

    // Start effect loop
    start_effect_loop(ctrl_state.clone(), frame_tx.clone());

    let app_state = AppState {
        ctrl: ctrl_state,
        frame_tx,
    };

    let app = Router::new()
        .route("/", get(serve_ui))
        .route("/api/status", get(get_status))
        .route("/api/presets", get(get_presets))
        .route("/api/preset", post(set_preset))
        .route("/api/brightness", post(set_brightness))
        .route("/api/frame", get(get_frame))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = "127.0.0.1:7070";
    println!("🌐  Listening on http://{}", addr);
    println!("    Open this URL in your browser to control the keyboard.");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
