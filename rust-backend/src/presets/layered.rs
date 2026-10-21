use crate::led_driver::{LedController, Color, NUM_ZONES};
use crate::effects::Effect;

pub struct EffectLayer {
    pub effect: Box<dyn Effect>,
    pub opacity: f32,
    pub priority: i32,
}

pub struct LayeredEffect {
    layers: Vec<EffectLayer>,
}

impl LayeredEffect {
    pub fn new(mut layers: Vec<EffectLayer>) -> Self {
        // Sort layers ascending by priority, so lower priority runs first and higher priority runs on top
        layers.sort_by_key(|l| l.priority);
        LayeredEffect { layers }
    }
}

impl Effect for LayeredEffect {
    fn start(&mut self) {
        for layer in &mut self.layers {
            layer.effect.start();
        }
    }

    fn update(&mut self, controller: &mut LedController, time: f32, delta: f32) {
        let mut accum_buffer = [Color::black(); NUM_ZONES];
        
        // We set suspend_flushing to true to avoid intermediate HID writes or latency delays
        controller.suspend_flushing = true;

        for (layer_idx, layer) in self.layers.iter_mut().enumerate() {
            // Clear buffer before running this layer
            controller.fill(Color::black());
            
            // Run the sub-effect update
            layer.effect.update(controller, time, delta);
            
            // Extract the result and blend it into accum_buffer
            let written = controller.get_buffer();
            let opacity = layer.opacity.clamp(0.0, 1.0);

            for i in 0..NUM_ZONES {
                let top = written[i];

                if layer_idx == 0 {
                    // Base layer: sets initial buffer
                    accum_buffer[i] = Color::black().lerp(&top, opacity);
                } else {
                    // Overlay layers: if top is black, preserve bottom effect;
                    // if top has color, override bottom effect set color.
                    if top.r == 0 && top.g == 0 && top.b == 0 {
                        continue;
                    }

                    let max_c = (top.r.max(top.g).max(top.b)) as f32;
                    let brightness = (max_c / 255.0).clamp(0.0, 1.0);

                    // Reconstruct full chromatic color so fading effects smoothly
                    // crossfade from the top color back into the bottom effect's color
                    let scale = 255.0 / max_c;
                    let full_chroma = Color::new(
                        (top.r as f32 * scale).round().clamp(0.0, 255.0) as u8,
                        (top.g as f32 * scale).round().clamp(0.0, 255.0) as u8,
                        (top.b as f32 * scale).round().clamp(0.0, 255.0) as u8,
                    );

                    let effective_alpha = (brightness * opacity).clamp(0.0, 1.0);
                    accum_buffer[i] = accum_buffer[i].lerp(&full_chroma, effective_alpha);
                }
            }
        }

        // Restore suspend_flushing
        controller.suspend_flushing = false;

        // Write the blended accum_buffer back to the controller
        for i in 0..NUM_ZONES {
            controller.set_zone(i, accum_buffer[i]);
        }

        // Flush the final blended frame to the hardware
        let _ = controller.flush_buffered();
    }

    fn stop(&mut self, controller: &mut LedController) {
        for layer in &mut self.layers {
            layer.effect.stop(controller);
        }
        let _ = controller.clear();
    }

    fn name(&self) -> &str {
        "Layered Effects"
    }
}
