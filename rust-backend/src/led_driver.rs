// led_driver.rs
// RGB Keyboard LED Driver
// Handles low-level communication with the RGB keyboard controller

use hidapi::{HidApi, HidDevice};

use std::sync::{Arc, Mutex};

const VID: u16 = 0x048d;
const PID: u16 = 0xc693;
pub const NUM_ZONES: usize = 24;


#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Color { 
    pub r: u8, 
    pub g: u8, 
    pub b: u8 
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self { 
        Color { r, g, b } 
    }
    
    pub fn black() -> Self { 
        Color::new(0, 0, 0) 
    }
    
    pub fn white() -> Self { 
        Color::new(255, 255, 255) 
    }
    
    pub fn red() -> Self { 
        Color::new(255, 0, 0) 
    }
    
    pub fn green() -> Self { 
        Color::new(0, 255, 0) 
    }
    
    pub fn blue() -> Self { 
        Color::new(0, 0, 255) 
    }
    
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h / 60.0;
        let i = h.floor() as i32 % 6;
        let f = h - h.floor();
        let p = v * (1.0 - s);
        let q = v * (1.0 - f * s);
        let t = v * (1.0 - (1.0 - f) * s);
        let (r, g, b) = match i {
            0 => (v, t, p), 
            1 => (q, v, p), 
            2 => (p, v, t),
            3 => (p, q, v), 
            4 => (t, p, v), 
            _ => (v, p, q),
        };
        Color::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }

    /// Legacy linear scale (kept for unit tests / internal uses)
    pub fn scale(&self, brightness: f32) -> Self {
        Color::new(
            (self.r as f32 * brightness.clamp(0.0, 1.0)) as u8,
            (self.g as f32 * brightness.clamp(0.0, 1.0)) as u8,
            (self.b as f32 * brightness.clamp(0.0, 1.0)) as u8,
        )
    }

    /// Perceptual brightness scaling to apply as the FINAL pass before HID write.
    /// Uses a simple gamma approximation (gamma = 2.2): multiplier = b^(1/2.2).
    /// This keeps perceived brightness approximately linear to the slider.
    pub fn perceptual_scale(&self, brightness: f32) -> Self {
        let b = brightness.clamp(0.0, 1.0);
        let m = if b <= 0.0 { 0.0 } else { b.powf(1.0 / 2.2) };
        Color::new(
            (self.r as f32 * m).round().clamp(0.0, 255.0) as u8,
            (self.g as f32 * m).round().clamp(0.0, 255.0) as u8,
            (self.b as f32 * m).round().clamp(0.0, 255.0) as u8,
        )
    }
    
    pub fn lerp(&self, other: &Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Color::new(
            (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
        )
    }
}


pub struct LedController {
    device: Option<HidDevice>,
    frame_buffer: [Color; NUM_ZONES],
    ui_frame: Arc<Mutex<Vec<Color>>>, //frontend-visible frame
    brightness: f32, // global brightness (0.0 - 1.0), applied as final pass
    pub suspend_flushing: bool,
}

impl LedController {
    pub fn new(ui_frame: Arc<Mutex<Vec<Color>>>) -> Self {
        LedController { 
            device: None, 
            frame_buffer: [Color::black(); NUM_ZONES],
            ui_frame,
            brightness: 1.0,
            suspend_flushing: false,
        }
        
    }

    /// Connect to the RGB keyboard controller (interface 1)
    pub fn connect(&mut self) -> Result<(), String> {
        self.connect_device(VID, PID)
    }

    /// Connect to the RGB keyboard controller with custom VID/PID (interface 1)
    pub fn connect_device(&mut self, vid: u16, pid: u16) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            // Kill any other running instances of rgb-server to release the HID device
            use sysinfo::System;
            let mut sys = System::new();
            sys.refresh_all();
            let my_pid = std::process::id();
            let mut killed_any = false;
            for (pid, process) in sys.processes() {
                if process.name() == "rgb-server" {
                    let pid_val = pid.to_string().parse::<u32>().unwrap_or(0);
                    if pid_val != my_pid && pid_val != 0 {
                        println!("[LedController] Stopping existing stale instance (PID {})...", pid_val);
                        process.kill();
                        killed_any = true;
                    }
                }
            }
            if killed_any {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }

        let api = HidApi::new().map_err(|e| e.to_string())?;
        
        // Find device with interface 1 (RGB controller)
        for device_info in api.device_list() {
            if device_info.vendor_id() == vid 
                && device_info.product_id() == pid 
                && device_info.interface_number() == 1 
            {
                self.device = api.open_path(device_info.path()).ok();
                if self.device.is_some() {
                    // Disable autonomous mode to take control of lighting
                    if let Err(e) = self.set_autonomous_mode(false) {
                        eprintln!("[LedController] Failed to disable autonomous mode: {}", e);
                    }
                    return Ok(());
                }
            }
        }
        
        Err(format!("LED device with VID {:04x} PID {:04x} not found on interface 1", vid, pid))
    }

    /// Set global brightness (0.0 - 1.0). This is applied only when writing to the device/UI frame.
    pub fn set_brightness(&mut self, b: f32) {
        self.brightness = b.clamp(0.0, 1.0);
    }

    /// Read current global brightness
    pub fn brightness(&self) -> f32 {
        self.brightness
    }

    /// Check if the controller is connected
    pub fn is_connected(&self) -> bool { 
        self.device.is_some() 
    }

    /// Disconnect from the device
    pub fn disconnect(&mut self) {
        if self.device.is_some() {
            let _ = self.set_autonomous_mode(true);
        }
        self.device = None;
    }

    /// Drop the current (potentially stale) HID handle and reconnect.
    /// Called automatically by the effect loop after detecting a resume from sleep.
    pub fn reconnect(&mut self) -> Result<(), String> {
        self.device = None;
        self.connect()
    }

    // ===================================================================
    // BUFFER MANAGEMENT
    // ===================================================================

    /// Set a specific zone color in the frame buffer (does not send to device)
    pub fn set_zone(&mut self, zone: usize, color: Color) {
        if zone < NUM_ZONES { 
            self.frame_buffer[zone] = color; 
        }
    }

    /// Get the color of a specific zone from the frame buffer
    pub fn get_zone(&self, zone: usize) -> Color {
        self.frame_buffer.get(zone).cloned().unwrap_or(Color::black())
    }
    
    /// Get a reference to the entire frame buffer
    pub fn get_buffer(&self) -> &[Color; NUM_ZONES] {
        &self.frame_buffer
    }
    
    /// Get the frame buffer as a Vec (useful for cloning/sending to frontend)
    pub fn get_buffer_vec(&self) -> Vec<Color> {
        self.frame_buffer.to_vec()
    }
    
    /// Set the entire frame buffer
    pub fn set_buffer(&mut self, buffer: [Color; NUM_ZONES]) {
        self.frame_buffer = buffer;
    }

    /// Fill the entire frame buffer with a single color (does not send to device)
    pub fn fill(&mut self, color: Color) {
        self.frame_buffer = [color; NUM_ZONES];
    }

    // ===================================================================
    // COMMAND 0x05: ZONE RANGE (Efficient for solid colors/ranges)
    // ===================================================================

    /// Set a range of zones to a specific color using command 0x05
    /// This command applies immediately and is more efficient for solid colors
    /// 
    /// Format: [0x05, 0x01, start_zone, 0x00, end_zone, 0x00, R, G, B, 0x01]
    /// 
    /// # Arguments
    /// * `start` - Starting zone (0-23)
    /// * `end` - Ending zone (0-23)
    /// * `color` - RGB color to apply
    pub fn set_range(&mut self, start: u8, end: u8, color: Color) -> Result<(), String> {
        if start > 23 || end > 23 || start > end {
            return Err(format!("Invalid zone range: {}-{}", start, end));
        }

        // Update internal frame buffer (logical color) to keep state synchronized
        for i in start..=end {
            self.frame_buffer[i as usize] = color;
        }

        if !self.suspend_flushing {
            let device = self.device.as_ref().ok_or("Not connected")?;
            // Compute perceptually-scaled color for the device/UI (do NOT replace logical buffer)
            let scaled = color.perceptual_scale(self.brightness);
            
            let buf = vec![
                0x05,     // Command: Vendor lighting
                0x01,     // Subcommand: Zone range RGB
                start,    // Start zone index
                0x00,     // Reserved (must be zero)
                end,      // End zone index
                0x00,     // Reserved (must be zero)
                scaled.r,  // Red (0-255)
                scaled.g,  // Green (0-255)
                scaled.b,  // Blue (0-255)
                0x01,     // Apply/Commit (1 = apply immediately)
            ];
            device.send_feature_report(&buf).map_err(|e| e.to_string())?;
            
            // Add a delay (10ms for commit/apply) to prevent overwhelming the ITE controller
            std::thread::sleep(std::time::Duration::from_millis(10));
            
            // Update frontend-visible frame with SCALED colors so the UI matches the device
            let mut frame = self.ui_frame.lock().unwrap();
            for i in start..=end {
                frame[i as usize] = scaled;
            }
        }
        Ok(())
    }
    
    /// Set all zones to a single color instantly using command 0x05
    pub fn set_all_instant(&mut self, color: Color) -> Result<(), String> {
        self.set_range(0, 23, color)
    }

    // ===================================================================
    // COMMAND 0x04: INDIVIDUAL ZONES (For complex patterns)
    // ===================================================================

    /// Send a packet with 8 individual zone colors using command 0x04
    /// 
    /// Format: [0x04, 0x08, commit_flag, zone_indices (16 bytes), color_data (32 bytes)]
    /// - Byte 0: 0x04 (Command)
    /// - Byte 1: 0x08 (Number of zones, always 8)
    /// - Byte 2: 0x00 or 0x01 (Commit flag - set to 1 for last packet)
    /// - Bytes 3-18: Zone indices (8 zones × 2 bytes: zone_id, 0x00)
    /// - Bytes 19-50: Color data (8 zones × 4 bytes: R, G, B, 0x01)
    /// 
    /// # Arguments
    /// * `zone_start` - Starting zone index (0, 8, or 16)
    /// * `colors` - Array of exactly 8 colors
    /// * `commit` - Set to true for the last packet to apply changes
    fn send_zone_packet(&self, zone_start: u8, colors: &[Color; 8], commit: bool) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Not connected")?;
        
        let mut buf = vec![
            0x04,                             // Command
            0x08,                             // Number of zones (always 8)
            if commit { 0x01 } else { 0x00 }, // Commit flag
        ];
        
        // Add zone indices (8 zones, 2 bytes each)
        for i in 0..8 {
            buf.push(zone_start + i);  // Zone ID
            buf.push(0x00);            // Spacer
        }
        
        // Add color data (8 zones, 4 bytes each)
        for color in colors {
            buf.push(color.r);  // Red
            buf.push(color.g);  // Green
            buf.push(color.b);  // Blue
            buf.push(0x01);     // Color commit bit
        }
        device.send_feature_report(&buf).map_err(|e| e.to_string())?;
        
        // Add a delay to prevent overwhelming the ITE controller (10ms after commit, 5ms between packets)
        let delay_ms = if commit { 10 } else { 5 };
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        
        Ok(())
    }

    /// Flush the frame buffer to the device using command 0x04
    /// Sends all 24 zones in 3 packets (8 zones each)
    /// The last packet has the commit flag set to apply changes
    pub fn flush_buffered(&self) -> Result<(), String> {
        if self.suspend_flushing {
            return Ok(());
        }
        // Prepare color arrays for each packet (logical colors)
        let mut colors_0_7 = [Color::black(); 8];
        let mut colors_8_15 = [Color::black(); 8];
        let mut colors_16_23 = [Color::black(); 8];
        
        colors_0_7.copy_from_slice(&self.frame_buffer[0..8]);
        colors_8_15.copy_from_slice(&self.frame_buffer[8..16]);
        colors_16_23.copy_from_slice(&self.frame_buffer[16..24]);

        // Create SCALED copies for sending (perceptual scaling)
        let mut scaled_0_7 = [Color::black(); 8];
        let mut scaled_8_15 = [Color::black(); 8];
        let mut scaled_16_23 = [Color::black(); 8];

        for i in 0..8 {
            scaled_0_7[i] = colors_0_7[i].perceptual_scale(self.brightness);
            scaled_8_15[i] = colors_8_15[i].perceptual_scale(self.brightness);
            scaled_16_23[i] = colors_16_23[i].perceptual_scale(self.brightness);
        }

        // Send zones using SCALED data
        self.send_zone_packet(0, &scaled_0_7, false)?;
        self.send_zone_packet(8, &scaled_8_15, false)?;
        self.send_zone_packet(16, &scaled_16_23, true)?;

        // Update frontend-visible frame with SCALED colors (so UI matches device)
        let mut frame = self.ui_frame.lock().unwrap();
        if frame.len() == 24 {
            for i in 0..8 {
                frame[i] = scaled_0_7[i];
                frame[8 + i] = scaled_8_15[i];
                frame[16 + i] = scaled_16_23[i];
            }
        } else {
            frame.clear();
            frame.extend_from_slice(&[
                scaled_0_7[0], scaled_0_7[1], scaled_0_7[2], scaled_0_7[3], scaled_0_7[4], scaled_0_7[5], scaled_0_7[6], scaled_0_7[7],
                scaled_8_15[0], scaled_8_15[1], scaled_8_15[2], scaled_8_15[3], scaled_8_15[4], scaled_8_15[5], scaled_8_15[6], scaled_8_15[7],
                scaled_16_23[0], scaled_16_23[1], scaled_16_23[2], scaled_16_23[3], scaled_16_23[4], scaled_16_23[5], scaled_16_23[6], scaled_16_23[7],
            ]);
        }
        
        Ok(())
    }

    /// Clear all zones (set to black) using command 0x05
    pub fn clear(&mut self) -> Result<(), String> {
        self.frame_buffer = [Color::black(); NUM_ZONES];
        if !self.suspend_flushing {
            self.set_all_instant(Color::black())?;
        }
        Ok(())
    }

    /// Enable or disable the controller's autonomous (firmware-driven) lighting mode.
    /// Setting this to false allows the host to drive custom per-zone colors.
    pub fn set_autonomous_mode(&self, autonomous: bool) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Not connected")?;
        let buf = vec![
            0x06, // Report ID 6 (LampArrayControlReport)
            if autonomous { 0x01 } else { 0x00 }, // 0x01 = autonomous, 0x00 = host-controlled
        ];
        device.send_feature_report(&buf).map_err(|e| e.to_string())?;
        
        // Short delay to let the controller register the mode switch
        std::thread::sleep(std::time::Duration::from_millis(10));
        Ok(())
    }
}

impl Drop for LedController {
    fn drop(&mut self) {
        // Re-enable autonomous (hardware default) mode upon dropping the controller
        self.disconnect();
    }
}



// ===================================================================
// TESTS
// ===================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let red = Color::red();
        assert_eq!(red.r, 255);
        assert_eq!(red.g, 0);
        assert_eq!(red.b, 0);
    }

    #[test]
    fn test_color_hsv() {
        let red = Color::from_hsv(0.0, 1.0, 1.0);
        assert_eq!(red.r, 255);
        
        let green = Color::from_hsv(120.0, 1.0, 1.0);
        assert_eq!(green.g, 255);
        
        let blue = Color::from_hsv(240.0, 1.0, 1.0);
        assert_eq!(blue.b, 255);
    }

    #[test]
    fn test_color_scale() {
        let white = Color::white();
        let half = white.scale(0.5);
        assert!(half.r > 120 && half.r < 130);
    }

    #[test]
    fn test_perceptual_scale() {
        // perceptual scaling at 0.5 should be noticeably brighter than linear 0.5
        let white = Color::white();
        let p = white.perceptual_scale(0.5);
        // Expect approx 186 (0.5^(1/2.2) * 255 ≈ 186)
        assert!(p.r > 180 && p.r < 195, "perceptual scale produced {}", p.r);
    }

    #[test]
    fn test_color_lerp() {
        let black = Color::black();
        let white = Color::white();
        let gray = black.lerp(&white, 0.5);
        assert!(gray.r > 120 && gray.r < 130);
    }

    #[test]
    fn test_buffer_operations() {
        let ui_frame = Arc::new(Mutex::new(vec![Color::black(); NUM_ZONES]));
        let mut controller = LedController::new(ui_frame);
        
        controller.set_zone(0, Color::red());
        assert_eq!(controller.get_zone(0), Color::red());
        
        controller.fill(Color::blue());
        assert_eq!(controller.get_zone(0), Color::blue());
        assert_eq!(controller.get_zone(23), Color::blue());
    }
}