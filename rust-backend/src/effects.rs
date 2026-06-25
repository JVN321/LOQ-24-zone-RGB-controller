// effects.rs
// Effects system template for RGB keyboard
// Each effect should implement the Effect trait

use crate::led_driver::{LedController, NUM_ZONES};

/// Effect trait that all effects must implement
pub trait Effect: Send {

    /// Called once when the effect is activated
    fn start(&mut self) {}
    
    /// Called every frame to update the effect
    /// 
    /// # Arguments
    /// * `controller` - LED controller to manipulate
    /// * `time` - Total time since effect started (seconds)
    /// * `delta` - Time since last update (seconds)
    fn update(&mut self, controller: &mut LedController, time: f32, delta: f32);
    
    /// Called once when the effect is stopped
    fn stop(&mut self, controller: &mut LedController) {
        let _ = controller.clear();
    }
    
    /// Get effect name (for debugging/UI)
    fn name(&self) -> &str {
        "Unknown Effect"
    }
}

// ===================================================================
// HELPER FUNCTIONS FOR EFFECTS
// ===================================================================

/// Get distance from center zone
pub fn distance_from_center(zone: usize) -> f32 {
    let center = NUM_ZONES as f32 / 2.0;
    (zone as f32 - center).abs() / center
}

/// Smooth step interpolation (ease in/out)
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}