use crate::led_driver::{LedController, Color};
use crate::effects::Effect;

pub struct StaticEffect {
    colors: [Color; 24],
}

impl StaticEffect {
    pub fn new(colors: [Color; 24]) -> Self {
        StaticEffect { colors }
    }
}

impl Effect for StaticEffect {
    fn update(&mut self, controller: &mut LedController, _time: f32, _delta: f32) {
        for i in 0..24 {
            controller.set_zone(i, self.colors[i]);
        }
        let _ = controller.flush_buffered();
    }

    fn name(&self) -> &str {
        "Static"
    }
}