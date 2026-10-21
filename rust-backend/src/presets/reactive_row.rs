use crate::led_driver::{LedController, Color, NUM_ZONES};
use crate::effects::Effect;
use crate::input_handler::KEY_EVENTS;

/// Color modes for Reactive Row:
/// 0 = Solid Color (custom chosen color)
/// 1 = Keystroke Hue Shift (every key typed shifts the color to the next spectrum hue)
/// 2 = Flowing Rainbow Wave (spatial rainbow animation across active zones)
/// 3 = Keyboard Spectrum (fixed 360° chromatic mapping across zones 0-23)
/// 4 = Random Color Pop (each keystroke triggers a distinct randomized vivid color)

fn pseudo_random_hue(seed: f32, zone: usize) -> f32 {
    let bits = (seed * 1000.0) as u32 ^ (zone as u32 * 2654435761);
    let h = bits.wrapping_mul(0x45d9f3b);
    let h = ((h >> 16) ^ h).wrapping_mul(0x45d9f3b);
    let h = (h >> 16) ^ h;
    (h % 360) as f32
}

pub struct ReactiveRowEffect {
    last_press_time: [f32; NUM_ZONES],
    zone_hues: [f32; NUM_ZONES],
    current_global_hue: f32,
    color_mode: u8,
    color: Color,
    bg_color: Color,
    hold_time: f32,
    fade_time: f32,
    width: f32,
    color_speed: f32,
    invert: bool,
}

impl ReactiveRowEffect {
    pub fn new(
        color_mode: u8,
        color: Color,
        bg_color: Color,
        hold_time: f32,
        fade_time: f32,
        width: f32,
        color_speed: f32,
        invert: bool,
    ) -> Self {
        ReactiveRowEffect {
            last_press_time: [-1000.0; NUM_ZONES],
            zone_hues: [0.0; NUM_ZONES],
            current_global_hue: 0.0,
            color_mode,
            color,
            bg_color,
            hold_time: hold_time.max(0.0),
            fade_time: fade_time.max(0.01),
            width: width.max(1.0),
            color_speed: color_speed.max(0.0),
            invert,
        }
    }
}

impl Effect for ReactiveRowEffect {
    fn start(&mut self) {
        self.last_press_time = [-1000.0; NUM_ZONES];
        self.zone_hues = [0.0; NUM_ZONES];
        self.current_global_hue = 0.0;
    }

    fn update(&mut self, controller: &mut LedController, time: f32, _delta: f32) {
        // 1. Drain recent keypress events
        if let Ok(mut events) = KEY_EVENTS.lock() {
            for zone in events.drain(..) {
                let z = (zone as usize).min(NUM_ZONES - 1);
                self.last_press_time[z] = time;

                // Shift global hue per keypress (e.g. 25 deg per step scaled by speed)
                let step = (self.color_speed.max(0.1) * 25.0).clamp(5.0, 120.0);
                self.current_global_hue = (self.current_global_hue + step) % 360.0;

                self.zone_hues[z] = match self.color_mode {
                    1 => self.current_global_hue,
                    4 => pseudo_random_hue(time, z),
                    _ => self.current_global_hue,
                };
            }
        }

        // 2. Compute illumination intensity and dominant color source for each zone
        for i in 0..NUM_ZONES {
            let mut intensity = 0.0f32;
            let mut dominant_source = i;

            if self.width <= 1.0 {
                let dt = time - self.last_press_time[i];
                if dt >= 0.0 && dt < self.hold_time + self.fade_time {
                    intensity = if dt <= self.hold_time {
                        1.0
                    } else {
                        1.0 - (dt - self.hold_time) / self.fade_time
                    };
                }
            } else {
                let w = self.width;
                let min_k = (i as f32 - w + 1.0).floor().max(0.0) as usize;
                let max_k = (i as f32 + w).floor().min((NUM_ZONES - 1) as f32) as usize;

                for k in min_k..=max_k {
                    let dt = time - self.last_press_time[k];
                    if dt >= 0.0 && dt < self.hold_time + self.fade_time {
                        let zone_k_intensity = if dt <= self.hold_time {
                            1.0
                        } else {
                            1.0 - (dt - self.hold_time) / self.fade_time
                        };
                        let dist = (i as f32 - k as f32).abs();
                        if dist < w {
                            let falloff = 1.0 - (dist / w);
                            let eff = zone_k_intensity * falloff;
                            if eff > intensity {
                                intensity = eff;
                                dominant_source = k;
                            }
                        }
                    }
                }
            }

            // 3. Inversion logic:
            // - Normal: idle is 0.0 (bg_color), keypress activates row towards 1.0 (active_color)
            // - Inverted: idle is 1.0 (active_color), keypress dims row towards 0.0 (bg_color)
            let final_factor = if self.invert {
                (1.0 - intensity).clamp(0.0, 1.0)
            } else {
                intensity.clamp(0.0, 1.0)
            };

            // 4. Calculate active color based on the selected color mode
            let active_color = match self.color_mode {
                0 => self.color, // Solid Color
                1 => {
                    // Keystroke Hue Shift: captures the hue assigned at keypress time,
                    // plus a smooth gentle evolution during the hold/fade period
                    let age = (time - self.last_press_time[dominant_source]).max(0.0);
                    let anim_shift = age * self.color_speed * 30.0;
                    let hue = (self.zone_hues[dominant_source] + anim_shift) % 360.0;
                    Color::from_hsv(hue, 1.0, 1.0)
                }
                2 => {
                    // Flowing Rainbow Wave across the keyboard
                    let hue = (time * self.color_speed * 60.0 + i as f32 * 15.0) % 360.0;
                    Color::from_hsv(hue, 1.0, 1.0)
                }
                3 => {
                    // Fixed Keyboard Spectrum (zone-based rainbow) with optional slow rotation
                    let base_hue = i as f32 * (360.0 / NUM_ZONES as f32);
                    let hue = (base_hue + time * self.color_speed * 20.0) % 360.0;
                    Color::from_hsv(hue, 1.0, 1.0)
                }
                4 => {
                    // Random Color Pop assigned per keystroke
                    let hue = self.zone_hues[dominant_source];
                    Color::from_hsv(hue, 1.0, 1.0)
                }
                _ => self.color,
            };

            // 5. Interpolate between background color and active color
            let final_color = self.bg_color.lerp(&active_color, final_factor);
            controller.set_zone(i, final_color);
        }

        let _ = controller.flush_buffered();
    }

    fn stop(&mut self, controller: &mut LedController) {
        self.last_press_time = [-1000.0; NUM_ZONES];
        let _ = controller.clear();
    }

    fn name(&self) -> &str {
        "Reactive Row"
    }
}
