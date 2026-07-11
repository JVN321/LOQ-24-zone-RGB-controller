use crate::led_driver::{LedController, Color, NUM_ZONES};
use crate::effects::Effect;
use crate::input_handler::KEY_EVENTS;

struct Ripple {
    center: f32,
    start_time: f32,
}

pub struct KeyboardWaveEffect {
    ripples: Vec<Ripple>,
    speed: f32,           // Speed of the background rainbow wave flow
    ripple_speed: f32,    // Speed of the water wave propagation
    width: f32,           // Width of the wavefront envelope
    lifetime: f32,        // Lifetime of the ripple in seconds
    base_brightness: f32, // Idle brightness of the background rainbow
}

impl KeyboardWaveEffect {
    pub fn new(speed: f32, ripple_speed: f32, width: f32, lifetime: f32, base_brightness: f32) -> Self {
        KeyboardWaveEffect {
            ripples: Vec::new(),
            speed,
            ripple_speed,
            width,
            lifetime,
            base_brightness: base_brightness.clamp(0.0, 1.0),
        }
    }
}

impl Effect for KeyboardWaveEffect {
    fn start(&mut self) {
        self.ripples.clear();
    }

    fn update(&mut self, controller: &mut LedController, time: f32, _delta: f32) {
        // 1. Consume new key events
        if let Ok(mut events) = KEY_EVENTS.lock() {
            for zone in events.drain(..) {
                // Prevent ripple explosion by limiting active ripples
                if self.ripples.len() < 16 {
                    self.ripples.push(Ripple {
                        center: zone as f32,
                        start_time: time,
                    });
                }
            }
        }

        // 2. Clean up old ripples
        self.ripples.retain(|r| time - r.start_time < self.lifetime);

        // 3. Render the combined flowing wave and reactive water ripples
        for i in 0..NUM_ZONES {
            // Background flowing rainbow color for this zone
            // A full spatial cycle spans 24 zones (15 degrees per zone)
            let base_hue = (time * self.speed * 60.0 + i as f32 * 15.0) % 360.0;
            
            let mut total_intensity = 0.0f32;
            let mut total_highlight = 0.0f32;

            for ripple in &self.ripples {
                let age = time - ripple.start_time;
                let dist = (i as f32 - ripple.center).abs();
                let wave_pos = age * self.ripple_speed;
                let diff = dist - wave_pos;

                // Sine wave oscillation for the water ring structure
                let wave = (diff * 1.5).cos();
                
                // Gaussian envelope centered at the propagating wavefront
                let envelope = (-(diff * diff) / (self.width * self.width)).exp();
                
                // Quadratic decay over time
                let decay = (1.0 - age / self.lifetime).powi(2);

                let intensity = (wave * 0.5 + 0.5) * envelope * decay;
                total_intensity += intensity;

                // Highlight/crest effect at the leading edge of the ripple
                if diff.abs() < self.width * 0.5 {
                    total_highlight += envelope * decay;
                }
            }

            // Cap the combined ripple intensities
            let intensity_clamp = total_intensity.min(1.0);
            let highlight_clamp = total_highlight.min(0.6);

            // Compute target brightness: lerp from base_brightness up to 1.0 based on ripple intensity
            let target_brightness = self.base_brightness + (1.0 - self.base_brightness) * intensity_clamp;

            // Generate full-vibrancy rainbow color at computed brightness
            let mut final_color = Color::from_hsv(base_hue, 1.0, target_brightness);

            // Blend with white for a bright "crest/splash" wavefront highlight
            if highlight_clamp > 0.01 {
                final_color = final_color.lerp(&Color::white(), highlight_clamp);
            }

            controller.set_zone(i, final_color);
        }

        let _ = controller.flush_buffered();
    }

    fn stop(&mut self, controller: &mut LedController) {
        self.ripples.clear();
        let _ = controller.clear();
    }

    fn name(&self) -> &str {
        "Keyboard Wave"
    }
}
