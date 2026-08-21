#![allow(non_snake_case)]

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ParameterConfig {
    pub name: String,
    pub label: String,
    pub param_type: ParameterType,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub step: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ParameterType {
    Float,
    #[serde(rename = "Color")]
    Color { r: u8, g: u8, b: u8 },
    String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetMetadata {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub parameters: Vec<ParameterConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetConfig {
    pub name: String,
    pub parameters: HashMap<String, ParameterValue>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "value")]
pub enum ParameterValue {
    Float(f32),
    Color { r: u8, g: u8, b: u8 },
    String(String),
}

impl ParameterValue {
    pub fn from_json(v: &serde_json::Value) -> Option<Self> {
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
}

pub fn get_available_presets() -> Vec<PresetMetadata> {
    let mut static_colors_params = Vec::new();
    for i in 1..=24 {
        static_colors_params.push(ParameterConfig {
            name: format!("color{}", i),
            label: format!("Zone {}", i),
            param_type: ParameterType::Color { r: 255, g: 255, b: 255 },
            min: 0.0,
            max: 0.0,
            default: 0.0,
            step: 0.0,
        });
    }

    vec![PresetMetadata {
            name: "staticColor".to_string(),
            display_name: "Static Color".to_string(),
            description: "Customize the color of each keyboard zone separately".to_string(),
            parameters: static_colors_params,
        },
        PresetMetadata {
            name: "thermalStatus".to_string(),
            display_name: "CPU-Mem-GPU usage status".to_string(),
            description: "Left => CPU, Middle => Memory, Right => GPU".to_string(),
            parameters: vec![],
        },
        PresetMetadata {
            name: "off".to_string(),
            display_name: "Off".to_string(),
            description: "Turn off all lighting".to_string(),
            parameters: vec![],
        },
        PresetMetadata {
            name: "ambient".to_string(),
            display_name: "Screen Ambiance light effect.".to_string(),
            description: "Mimics ambient light based on screen content with high-contrast color algorithms.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "algorithm".to_string(),
                    label: "Color Algorithm".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 6.0,
                    default: 0.0,
                    step: 1.0,
                },
                ParameterConfig {
                    name: "response_speed".to_string(),
                    label: "Response Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 1.0,
                    max: 50.0,
                    default: 15.0,
                    step: 1.0,
                },
                ParameterConfig {
                    name: "dynamic_mode".to_string(),
                    label: "Transition Dynamic".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 3.0,
                    default: 0.0,
                    step: 1.0,
                },
                ParameterConfig {
                    name: "saturation".to_string(),
                    label: "Saturation Boost".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.5,
                    max: 4.0,
                    default: 2.2,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "contrast".to_string(),
                    label: "Contrast Gamma".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.5,
                    max: 3.0,
                    default: 1.3,
                    step: 0.05,
                },
                ParameterConfig {
                    name: "black_cutoff".to_string(),
                    label: "Black Level Cutoff".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 0.4,
                    default: 0.08,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "min_brightness".to_string(),
                    label: "Minimum Brightness Floor".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 0.5,
                    default: 0.0,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "brightness_boost".to_string(),
                    label: "Brightness Gain".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.5,
                    max: 2.5,
                    default: 1.1,
                    step: 0.05,
                },
                ParameterConfig {
                    name: "noise_threshold".to_string(),
                    label: "Jitter Noise Gate".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 0.1,
                    default: 0.02,
                    step: 0.005,
                },
                ParameterConfig {
                    name: "rect_x".to_string(),
                    label: "Capture Rect X (Left)".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 1.0,
                    default: 0.0,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "rect_y".to_string(),
                    label: "Capture Rect Y (Top)".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 1.0,
                    default: 0.0,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "rect_width".to_string(),
                    label: "Capture Rect Width".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.01,
                    max: 1.0,
                    default: 1.0,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "rect_height".to_string(),
                    label: "Capture Rect Height".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.01,
                    max: 1.0,
                    default: 1.0,
                    step: 0.01,
                },
            ],
        },
        PresetMetadata {
            name: "breathing".to_string(),
            display_name: "Color Breath".to_string(),
            description: "Fade in → fade out.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 5.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "color".to_string(),
                    label: "Color".to_string(),
                    param_type: ParameterType::Color { r: 255, g: 0, b: 0 },
                    min: 0.0,  // Not used for colors
                    max: 255.0, // Not used for colors
                    default: 0.0, // Not used for colors
                    step: 1.0, // Not used for colors
                },
            ],
        },
        PresetMetadata {
            name: "pulse".to_string(),
            display_name: "Pulse Center".to_string(),
            description: "Pulsing effect from center".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 5.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "color".to_string(),
                    label: "Color".to_string(),
                    param_type: ParameterType::Color { r: 255, g: 0, b: 0 },
                    min: 0.0,  // Not used for colors
                    max: 255.0, // Not used for colors
                    default: 0.0, // Not used for colors
                    step: 1.0, // Not used for colors
                },
            ],
        },
        PresetMetadata {
            name: "horse".to_string(),
            display_name: "Horse Color".to_string(),
            description: "A sharp chaser segment racing across a solid base color — fast, focused, and minimal.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 5.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "length".to_string(),
                    label: "Length".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 5.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "base_color".to_string(),
                    label: "Base Color".to_string(),
                    param_type: ParameterType::Color { r: 81, g: 169, b: 158 },
                    min: 0.0,  // Not used for colors
                    max: 255.0, // Not used for colors
                    default: 0.0, // Not used for colors
                    step: 1.0, // Not used for colors
                },
                ParameterConfig {
                    name: "horse_color".to_string(),
                    label: "Horse Color".to_string(),
                    param_type: ParameterType::Color { r: 255, g: 0, b: 0 },
                    min: 0.0,  // Not used for colors
                    max: 255.0, // Not used for colors
                    default: 0.0, // Not used for colors
                    step: 1.0, // Not used for colors
                },
            ],
        },
        PresetMetadata {
            name: "horseCycle".to_string(),
            display_name: "Horse Cycle".to_string(),
            description: "A racing chaser over a smoothly color-cycling base, blending calm ambience with high-energy motion.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 5.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "length".to_string(),
                    label: "Length".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 5.0,
                    step: 0.1,
                },
            ],
        },
        PresetMetadata {
            name: "rpm".to_string(),
            display_name: "Ferrari RPM".to_string(),
            description: "Ferrari-like = fast, aggressive, red-dominant, precision motion — not rainbow fluff.".to_string(),
            parameters: vec![],
        },
        PresetMetadata {
            name: "rainbowBreath".to_string(),
            display_name: "Rainbow Breath".to_string(),
            description: "Whole keyboard breathes through rainbow hues".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.01,
                },
            ],
        },
        PresetMetadata {
            name: "rainbowCycle".to_string(),
            display_name: "Rainbow Cycle".to_string(),
            description: "Whole keyboard cycles through hues together.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.01,
                },
            ],
        },
        PresetMetadata {
            name: "rainbowWave".to_string(),
            display_name: "Rainbow Wave".to_string(),
            description: "Left → right rainbow motion.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.01,
                },
            ],
        },
        PresetMetadata {
            name: "wheel".to_string(),
            display_name: "ColorWheelEffect".to_string(),
            description: "Each zone has a fixed hue offset → whole keyboard spins like a wheel".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 3.0,
                    default: 0.5,
                    step: 0.1,
                },
            ],
        },
        PresetMetadata {
            name: "sweep".to_string(),
            display_name: "Color sweep".to_string(),
            description: "Cycles through all 16.7 million colors one per frame, completing a full loop in about 3.5 days.".to_string(),
            parameters: vec![],
        },
        
        PresetMetadata {
            name: "aurora".to_string(),
            display_name: "Aurora".to_string(),
            description: "Flowing aurora effect".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 3.0,
                    default: 0.5,
                    step: 0.1,
                },
            ],
        },
        PresetMetadata {
            name: "scan".to_string(),
            display_name: "Color Scan".to_string(),
            description: "Scanning color effect".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 3.0,
                    default: 1.0,
                    step: 0.1,
                },
            ],
        },
        PresetMetadata {
            name: "sparkle".to_string(),
            display_name: "Sparkle".to_string(),
            description: "Random sparkling effect".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "density".to_string(),
                    label: "Density".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.01,
                    max: 0.5,
                    default: 0.1,
                    step: 0.01,
                },
            ],
        },
        PresetMetadata {
            name: "audio_sparkle".to_string(),
            display_name: "Audio Sparkle".to_string(),
            description: "Keyboard lights sparkle in sync with audio source.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "sensitivity".to_string(),
                    label: "Sensitivity".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "base_density".to_string(),
                    label: "Base Density".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 0.5,
                    default: 0.05,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "audio_source".to_string(),
                    label: "Audio Source (0=Sys, 1=Mic, 2=Both)".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 2.0,
                    default: 0.0,
                    step: 1.0,
                },
            ],
        },
        PresetMetadata {
            name: "audio_sparkle_rainbow".to_string(),
            display_name: "Audio Sparkle Rainbow".to_string(),
            description: "Rainbow sparkles that react to audio source.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "sensitivity".to_string(),
                    label: "Sensitivity".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "base_density".to_string(),
                    label: "Base Density".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 0.5,
                    default: 0.0,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "rainbow_speed".to_string(),
                    label: "Rainbow Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 5.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "audio_source".to_string(),
                    label: "Audio Source (0=Sys, 1=Mic, 2=Both)".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 2.0,
                    default: 0.0,
                    step: 1.0,
                },
            ],
        },
        PresetMetadata {
            name: "audio_sparkle_media".to_string(),
            display_name: "Audio Sparkle Media".to_string(),
            description: "Sparkles that match screen colors and react to audio source.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "sensitivity".to_string(),
                    label: "Sensitivity".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "base_density".to_string(),
                    label: "Base Density".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 0.5,
                    default: 0.0,
                    step: 0.01,
                },
                ParameterConfig {
                    name: "audio_source".to_string(),
                    label: "Audio Source (0=Sys, 1=Mic, 2=Both)".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 2.0,
                    default: 0.0,
                    step: 1.0,
                },
            ],
        },
        PresetMetadata {
            name: "audio_ripple".to_string(),
            display_name: "Audio Ripple".to_string(),
            description: "Ripples flash from the center on audio beats.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "sensitivity".to_string(),
                    label: "Sensitivity".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 10.0,
                    max: 100.0,
                    default: 40.0,
                    step: 1.0,
                },
                ParameterConfig {
                    name: "width".to_string(),
                    label: "Width".to_string(),
                    param_type: ParameterType::Float,
                    min: 1.0,
                    max: 10.0,
                    default: 3.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "lifetime".to_string(),
                    label: "Lifetime".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 2.0,
                    default: 0.8,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "audio_source".to_string(),
                    label: "Audio Source (0=Sys, 1=Mic, 2=Both)".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 2.0,
                    default: 0.0,
                    step: 1.0,
                },
            ],
        },
        PresetMetadata {
            name: "rainbow_ripple".to_string(),
            display_name: "Typing Rainbow Ripple".to_string(),
            description: "Rainbow waves that expand from the keys you press.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 10.0,
                    max: 100.0,
                    default: 40.0,
                    step: 1.0,
                },
                ParameterConfig {
                    name: "width".to_string(),
                    label: "Width".to_string(),
                    param_type: ParameterType::Float,
                    min: 1.0,
                    max: 10.0,
                    default: 3.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "lifetime".to_string(),
                    label: "Lifetime".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 2.0,
                    default: 0.8,
                    step: 0.1,
                },
            ],
        },
        PresetMetadata {
            name: "nebula".to_string(),
            display_name: "Nebula".to_string(),
            description: "Soft, atmospheric, zero harsh transitions.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.01,
                },
            ],
        },
        
        PresetMetadata {
            name: "chromaticBreath".to_string(),
            display_name: "Chromatic Breath".to_string(),
            description: "Extremely clean, perfect for idle mode.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.01,
                },
            ],
        },
        PresetMetadata {
            name: "layered".to_string(),
            display_name: "Layered Effects".to_string(),
            description: "Layer multiple effects on top of each other with priority and opacity/intensity".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "config".to_string(),
                    label: "Config JSON".to_string(),
                    param_type: ParameterType::String,
                    min: 0.0,
                    max: 0.0,
                    default: 0.0,
                    step: 0.0,
                }
            ],
        },
        PresetMetadata {
            name: "keyboard_wave".to_string(),
            display_name: "Keyboard Wave".to_string(),
            description: "Flowing RGB rainbow wave that ripples like water wherever you type.".to_string(),
            parameters: vec![
                ParameterConfig {
                    name: "speed".to_string(),
                    label: "Wave Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 10.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "ripple_speed".to_string(),
                    label: "Ripple Speed".to_string(),
                    param_type: ParameterType::Float,
                    min: 5.0,
                    max: 100.0,
                    default: 20.0,
                    step: 0.5,
                },
                ParameterConfig {
                    name: "width".to_string(),
                    label: "Ripple Width".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.5,
                    max: 10.0,
                    default: 3.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "lifetime".to_string(),
                    label: "Ripple Lifetime (sec)".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.1,
                    max: 5.0,
                    default: 1.0,
                    step: 0.1,
                },
                ParameterConfig {
                    name: "base_brightness".to_string(),
                    label: "Background Brightness".to_string(),
                    param_type: ParameterType::Float,
                    min: 0.0,
                    max: 1.0,
                    default: 0.20,
                    step: 0.05,
                },
            ],
        },
    ]
}

pub fn get_preset_metadata() -> Vec<PresetMetadata> {
    get_available_presets()
}

pub mod off;
pub mod pulse;
pub mod scan;
pub mod sparkle;
pub mod aurora;
pub mod nebula;
pub mod chromaticBreath;
pub mod staticColor;
pub mod rainbowCycle;
pub mod rainbowWave;
pub mod breathing;
pub mod rainbowBreath;
pub mod wheel;
pub mod sweep;
pub mod horse;
pub mod horseCycle;
pub mod rpm;
pub mod thermalStatus;
pub mod ambient;
pub mod audio_sparkle;
pub mod audio_sparkle_rainbow;
pub mod audio_sparkle_media;
pub mod rainbow_ripple;
pub mod audio_ripple;
pub mod layered;
pub mod keyboard_wave;