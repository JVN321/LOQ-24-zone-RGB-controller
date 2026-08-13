#![allow(non_snake_case)]

use crate::led_driver::{LedController, Color, NUM_ZONES};

//
// ================= CONFIG =================
//

const AMBIENT_WIDTH: usize = NUM_ZONES;
const AMBIENT_HEIGHT: usize = 12;
/// Boost saturation so colors stay rich instead of washing to white (1.0 = no change).
const SATURATION_BOOST: f32 = 2.8;

//
// ================= RGB FLOAT =================
//

#[derive(Copy, Clone)]
pub struct RgbF {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl RgbF {
    pub fn black() -> Self {
        Self { r: 0.0, g: 0.0, b: 0.0 }
    }

    pub fn add(self, o: Self) -> Self {
        Self { r: self.r + o.r, g: self.g + o.g, b: self.b + o.b }
    }

    pub fn sub(self, o: Self) -> Self {
        Self { r: self.r - o.r, g: self.g - o.g, b: self.b - o.b }
    }

    pub fn scale(self, s: f32) -> Self {
        Self { r: self.r * s, g: self.g * s, b: self.b * s }
    }



    pub fn to_color(self) -> Color {
        Color::new(
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    /// Boost saturation: pull color away from gray so it stays rich, not white.
    fn saturate(self, _amount: f32) -> Self {
        let r = self.r;
        let g = self.g;
        let b = self.b;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let h = if h < 0.0 { h + 360.0 } else { h };
        let s = if max == 0.0 { 0.0 } else { delta / max };
        let v = max;

        // Apply a strong non-linear saturation curve so even slight hints of color become very vibrant.
        // Very dark pixels with any hue should still show that hue on the keyboard.
        let s_boosted = if s > 0.001 {
            (s.powf(0.2) * 2.0).min(1.0)
        } else {
            0.0
        };

        // Boost Value (brightness) so dim content is still visible on the keyboard.
        // We raise the floor to at least 0.20 so totally dark zones don't disappear entirely.
        const MIN_BRIGHTNESS: f32 = 0.20;
        let v_boosted = if v > 0.001 {
            (v.powf(0.6) * 1.3).max(MIN_BRIGHTNESS).min(1.0)
        } else {
            0.0 // truly black — no hue to show, leave dark
        };

        // Convert back to RGB
        let c = v_boosted * s_boosted;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v_boosted - c;

        let (r1, g1, b1) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Self {
            r: (r1 + m).clamp(0.0, 1.0),
            g: (g1 + m).clamp(0.0, 1.0),
            b: (b1 + m).clamp(0.0, 1.0),
        }
    }

}

//
// ================= SCREEN SAMPLER TRAIT =================
//

pub trait ScreenSampler: Send {
    fn sample(&mut self, out: &mut [[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]) -> bool;
}

//
// ================= DXGI SAMPLER (WINDOWS) =================
//

#[cfg(target_os = "windows")]
mod dxgi {
    use super::*;
    use windows::{
        core::*,
        Win32::{
            Foundation::*,
            Graphics::{
                Direct3D::*,
                Direct3D11::*,
                Dxgi::*,
                Dxgi::Common::*,
            },
        },
    };

    pub struct DxgiScreenSampler {
        device: ID3D11Device,
        context: ID3D11DeviceContext,
        output: IDXGIOutput1,
        duplication: IDXGIOutputDuplication,
        staging: ID3D11Texture2D,
        width: u32,
        height: u32,
        rect_x: f32,
        rect_y: f32,
        rect_width: f32,
        rect_height: f32,
    }

    impl DxgiScreenSampler {
        pub fn new() -> anyhow::Result<Self> {
            unsafe {
                let dxgi_factory: IDXGIFactory1 = CreateDXGIFactory1()?;
                
                let mut adapter_index = 0;
                while let Ok(adapter) = dxgi_factory.EnumAdapters1(adapter_index) {
                    adapter_index += 1;
                    
                    let mut device = None;
                    let mut context = None;

                    if D3D11CreateDevice(
                        &adapter,
                        D3D_DRIVER_TYPE_UNKNOWN, // Must be UNKNOWN when an adapter is specified
                        HMODULE(0),
                        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                        None::<&[_]>,
                        D3D11_SDK_VERSION,
                        Some(&mut device),
                        None,
                        Some(&mut context),
                    ).is_err() {
                        continue;
                    }
                    
                    if let (Some(dev), Some(ctx)) = (device, context) {
                        let mut output_index = 0;
                        while let Ok(output) = adapter.EnumOutputs(output_index) {
                            output_index += 1;
                            
                            if let Ok(output1) = output.cast::<IDXGIOutput1>() {
                                // Try to duplicate output. On laptops, this only succeeds on the GPU driving the display.
                                if let Ok(duplication) = output1.DuplicateOutput(&dev) {
                                    let mut dupl_desc = std::mem::MaybeUninit::<DXGI_OUTDUPL_DESC>::zeroed();
                                    duplication.GetDesc(dupl_desc.as_mut_ptr());
                                    let dupl_desc = dupl_desc.assume_init();
                                    let width = dupl_desc.ModeDesc.Width;
                                    let height = dupl_desc.ModeDesc.Height;

                                    let staging_desc = D3D11_TEXTURE2D_DESC {
                                        Width: width,
                                        Height: height,
                                        MipLevels: 1,
                                        ArraySize: 1,
                                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                                        Usage: D3D11_USAGE_STAGING,
                                        BindFlags: 0,
                                        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                                        MiscFlags: 0,
                                    };

                                    let mut staging = None;
                                    dev.CreateTexture2D(&staging_desc, None, Some(&mut staging))?;
                                    
                                    if let Some(staging_tex) = staging {
                                        return Ok(Self {
                                            device: dev,
                                            context: ctx,
                                            output: output1,
                                            duplication,
                                            staging: staging_tex,
                                            width,
                                            height,
                                            rect_x: 0.0,
                                            rect_y: 0.0,
                                            rect_width: 1.0,
                                            rect_height: 1.0,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                Err(anyhow::anyhow!("Failed to find an active DXGI adapter and output for screen capture"))
            }
        }

        fn recreate_duplication(&mut self) -> anyhow::Result<()> {
            unsafe {
                let duplication = self.output.DuplicateOutput(&self.device)?;
                let mut dupl_desc = std::mem::MaybeUninit::<DXGI_OUTDUPL_DESC>::zeroed();
                duplication.GetDesc(dupl_desc.as_mut_ptr());
                let dupl_desc = dupl_desc.assume_init();
                let width = dupl_desc.ModeDesc.Width;
                let height = dupl_desc.ModeDesc.Height;

                if width != self.width || height != self.height {
                    let staging_desc = D3D11_TEXTURE2D_DESC {
                        Width: width,
                        Height: height,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_STAGING,
                        BindFlags: 0,
                        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                        MiscFlags: 0,
                    };

                    let mut staging = None;
                    self.device.CreateTexture2D(&staging_desc, None, Some(&mut staging))?;
                    if let Some(staging_tex) = staging {
                        self.staging = staging_tex;
                        self.width = width;
                        self.height = height;
                    }
                }

                self.duplication = duplication;
                Ok(())
            }
        }

        pub fn set_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
            self.rect_x = x.clamp(0.0, 1.0);
            self.rect_y = y.clamp(0.0, 1.0);
            self.rect_width = w.clamp(0.01, 1.0);
            self.rect_height = h.clamp(0.01, 1.0);
        }

        /// Set the vertical start as a fraction [0.0..1.0] from the top of the screen
        pub fn set_sample_top_fraction(&mut self, f: f32) {
            self.rect_y = f.clamp(0.0, 1.0);
            self.rect_height = (1.0 - self.rect_y).clamp(0.01, 1.0);
        }

        /// Set the horizontal sampling region using left offset and width (fractions)
        pub fn set_sample_horizontal_region(&mut self, left_frac: f32, width_frac: f32) {
            let left = left_frac.clamp(0.0, 1.0);
            let width = width_frac.clamp(0.0, 1.0);
            self.rect_x = left;
            self.rect_width = if left + width > 1.0 { 1.0 - left } else { width };
        }
    }

    unsafe fn dominant_color_in_zone_raw(
        data: *const u8,
        pitch: usize,
        width: usize,
        height: usize,
        start_x: usize,
        end_x:   usize,
        start_y: usize,
        end_y:   usize,
    ) -> RgbF {
        const QUANT: usize = 8;
        const BUCKETS: usize = 32; // 256 / 8

        let mut counts = [0u32; BUCKETS * BUCKETS * BUCKETS];

        let band_w = (end_x - start_x).max(1);
        let band_h = (end_y - start_y).max(1);
        let x_step = (band_w / 16).max(1);
        let y_step = (band_h / 16).max(1);

        for sy in (start_y..end_y).step_by(y_step) {
            let sy_clamped = sy.min(height - 1);
            for sx in (start_x..end_x).step_by(x_step) {
                let sx_clamped = sx.min(width - 1);
                let p = data.add(sy_clamped * pitch + sx_clamped * 4);
                // BGRA format: B is at index 0, G is at index 1, R is at index 2
                let ri = (*p.add(2) as usize) / QUANT;
                let gi = (*p.add(1) as usize) / QUANT;
                let bi = (*p.add(0) as usize) / QUANT;
                counts[ri * BUCKETS * BUCKETS + gi * BUCKETS + bi] += 1;
            }
        }

        let mut best_non_black_idx = None;
        let mut best_non_black_count = 0;
        let mut best_overall_idx = 0;
        let mut best_overall_count = 0;

        for (idx, &count) in counts.iter().enumerate() {
            if count == 0 {
                continue;
            }
            if count > best_overall_count {
                best_overall_count = count;
                best_overall_idx = idx;
            }

            let ri = idx / (BUCKETS * BUCKETS);
            let gi = (idx / BUCKETS) % BUCKETS;
            let bi = idx % BUCKETS;

            let r = ri as f32 / 31.0;
            let g = gi as f32 / 31.0;
            let b = bi as f32 / 31.0;

            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let chroma = max - min;

            // Classify as black or dark/desaturated gray:
            // - max < 0.12: Very dark pixels (black / dark gray)
            // - chroma < 0.15 && max < 0.5: Grayscale/desaturated backgrounds (middle dark grays)
            let is_black_or_gray = max < 0.12 || (chroma < 0.15 && max < 0.5);

            if !is_black_or_gray {
                if count > best_non_black_count {
                    best_non_black_count = count;
                    best_non_black_idx = Some(idx);
                }
            }
        }

        let best_idx = best_non_black_idx.unwrap_or(best_overall_idx);

        let ri = best_idx / (BUCKETS * BUCKETS);
        let gi = (best_idx / BUCKETS) % BUCKETS;
        let bi = best_idx % BUCKETS;

        RgbF {
            r: (ri as f32 + 0.5) * QUANT as f32 / 255.0,
            g: (gi as f32 + 0.5) * QUANT as f32 / 255.0,
            b: (bi as f32 + 0.5) * QUANT as f32 / 255.0,
        }
    }

    impl ScreenSampler for DxgiScreenSampler {
        fn sample(&mut self, out: &mut [[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]) -> bool {
            unsafe {
                let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut resource = None;

                // Try to acquire frame - return false if no new frame available
                if let Err(e) = self.duplication.AcquireNextFrame(0, &mut frame_info, &mut resource) {
                    if e.code() != DXGI_ERROR_WAIT_TIMEOUT {
                        let _ = self.recreate_duplication();
                    }
                    return false;
                }

                // Check if there was actually an update
                let has_update = frame_info.LastPresentTime != 0 || frame_info.AccumulatedFrames > 0;

                let texture: ID3D11Texture2D = resource.unwrap().cast().unwrap();
                self.context.CopyResource(&self.staging, &texture);

                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                if self.context.Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).is_ok() {
                    let data = mapped.pData as *const u8;
                    let pitch = mapped.RowPitch as usize;

                    // compute sampling region in pixels using configured fractions
                    let region_left = (self.width as f32 * self.rect_x) as usize;
                    let region_top = (self.height as f32 * self.rect_y) as usize;
                    let region_width = ((self.width as f32) * self.rect_width).max(1.0) as usize;
                    let region_height = ((self.height as f32) * self.rect_height).max(1.0) as usize;
                    let region_end_y = (region_top + region_height).min(self.height as usize);

                    let col_width = region_width as f32 / AMBIENT_WIDTH as f32;
                    for x in 0..AMBIENT_WIDTH {
                        let start_x = (region_left as f32 + x as f32 * col_width) as usize;
                        let end_x = (region_left as f32 + (x + 1) as f32 * col_width) as usize;
                        let end_x = end_x.max(start_x + 1).min(self.width as usize);

                        let dominant = dominant_color_in_zone_raw(
                            data,
                            pitch,
                            self.width as usize,
                            self.height as usize,
                            start_x,
                            end_x,
                            region_top,
                            region_end_y,
                        );

                        for y in 0..AMBIENT_HEIGHT {
                            out[x][y] = dominant;
                        }
                    }

                    self.context.Unmap(&self.staging, 0);
                }

                let _ = self.duplication.ReleaseFrame();
                
                // Return whether this was a real update
                has_update
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use dxgi::DxgiScreenSampler;

#[cfg(not(target_os = "windows"))]
mod linux_sampler {
    use super::*;
    use xcap::image::{ImageBuffer, Rgba};

    /// Capture the screen using `grim` (wlr-screencopy, no portal prompt).
    /// Returns raw RGBA bytes and (width, height) on success.
    fn capture_with_grim() -> Option<(ImageBuffer<Rgba<u8>, Vec<u8>>, u32, u32)> {
        let mut cmd = std::process::Command::new("grim");
        cmd.args(["-t", "png", "-"]);

        // Ensure XDG_RUNTIME_DIR is set
        let runtime_dir = match std::env::var("XDG_RUNTIME_DIR") {
            Ok(d) if !d.trim().is_empty() => d,
            _ => {
                let uid = std::process::Command::new("id")
                    .arg("-u")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "1000".to_string());
                let default_dir = format!("/run/user/{}", uid);
                cmd.env("XDG_RUNTIME_DIR", &default_dir);
                default_dir
            }
        };

        // Auto-detect WAYLAND_DISPLAY if missing or empty in environment
        if std::env::var("WAYLAND_DISPLAY").map(|v| v.trim().is_empty()).unwrap_or(true) {
            if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
                let mut sockets: Vec<String> = entries
                    .flatten()
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with("wayland-") {
                            Some(name)
                        } else {
                            None
                        }
                    })
                    .collect();
                sockets.sort();
                if let Some(socket) = sockets.last() {
                    cmd.env("WAYLAND_DISPLAY", socket);
                    std::env::set_var("WAYLAND_DISPLAY", socket);
                }
            }
        }

        let output = match cmd.output() {
            Ok(out) => out,
            Err(e) => {
                static LAST_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let last = LAST_LOG.load(std::sync::atomic::Ordering::Relaxed);
                if now - last >= 5 {
                    LAST_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("⚠️  grim execution failed: {}", e);
                }
                return None;
            }
        };

        if !output.status.success() || output.stdout.is_empty() {
            // Display connection failed — clear WAYLAND_DISPLAY to force socket re-detection on next attempt
            std::env::remove_var("WAYLAND_DISPLAY");

            static LAST_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let last = LAST_LOG.load(std::sync::atomic::Ordering::Relaxed);
            if now - last >= 5 {
                LAST_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
                eprintln!(
                    "⚠️  grim failed ({}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
            return None;
        }

        let img = match xcap::image::load_from_memory_with_format(&output.stdout, xcap::image::ImageFormat::Png) {
            Ok(val) => val.into_rgba8(),
            Err(e) => {
                eprintln!("⚠️  Failed to decode grim PNG stdout: {}", e);
                return None;
            }
        };
        let w = img.width();
        let h = img.height();
        Some((img, w, h))
    }

    pub struct LinuxScreenSampler {
        rect_x: f32,
        rect_y: f32,
        rect_width: f32,
        rect_height: f32,
        /// Cached screen resolution (detected once, updated if it changes)
        screen_w: u32,
        screen_h: u32,
    }

    impl LinuxScreenSampler {
        pub fn new() -> anyhow::Result<Self> {
            // Do a quick probe capture to learn the screen resolution.
            let (screen_w, screen_h) = capture_with_grim()
                .map(|(_, w, h)| (w, h))
                .unwrap_or((1920, 1080));

            Ok(Self {
                rect_x: 0.0,
                rect_y: 0.0,
                rect_width: 1.0,
                rect_height: 1.0,
                screen_w,
                screen_h,
            })
        }

        pub fn set_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
            self.rect_x = x.clamp(0.0, 1.0);
            self.rect_y = y.clamp(0.0, 1.0);
            self.rect_width = w.clamp(0.01, 1.0);
            self.rect_height = h.clamp(0.01, 1.0);
        }

        pub fn set_sample_top_fraction(&mut self, f: f32) {
            self.rect_y = f.clamp(0.0, 1.0);
            self.rect_height = (1.0 - self.rect_y).clamp(0.01, 1.0);
        }

        pub fn set_sample_horizontal_region(&mut self, left_frac: f32, width_frac: f32) {
            let left = left_frac.clamp(0.0, 1.0);
            let width = width_frac.clamp(0.0, 1.0);
            self.rect_x = left;
            self.rect_width = if left + width > 1.0 { 1.0 - left } else { width };
        }
    }

    /// Find the most prominent color in a rectangular pixel region using
    /// 5-bit quantization (32 buckets per channel = 32,768 total buckets).
    /// Sub-samples the region for speed — captures ~16×16 grid of points.
    fn dominant_color_in_zone(
        img: &xcap::image::ImageBuffer<xcap::image::Rgba<u8>, Vec<u8>>,
        start_x: usize,
        end_x:   usize,
        start_y: usize,
        end_y:   usize,
    ) -> RgbF {
        // 5-bit quantization: divide each channel by 8 → 32 levels per channel.
        // Near-identical hues group into the same bucket, preserving real colors
        // rather than averaging them into a blended non-color.
        const QUANT: usize = 8;
        const BUCKETS: usize = 32; // 256 / 8

        let mut counts = [0u32; BUCKETS * BUCKETS * BUCKETS];

        // Sub-sample: at most ~16 steps per axis for performance.
        let band_w = (end_x - start_x).max(1);
        let band_h = (end_y - start_y).max(1);
        let x_step = (band_w / 16).max(1);
        let y_step = (band_h / 16).max(1);

        for sy in (start_y..end_y).step_by(y_step) {
            for sx in (start_x..end_x).step_by(x_step) {
                let p = img.get_pixel(sx as u32, sy as u32);
                let ri = (p[0] as usize) / QUANT;
                let gi = (p[1] as usize) / QUANT;
                let bi = (p[2] as usize) / QUANT;
                counts[ri * BUCKETS * BUCKETS + gi * BUCKETS + bi] += 1;
            }
        }

        let mut best_non_black_idx = None;
        let mut best_non_black_count = 0;
        let mut best_overall_idx = 0;
        let mut best_overall_count = 0;

        for (idx, &count) in counts.iter().enumerate() {
            if count == 0 {
                continue;
            }
            if count > best_overall_count {
                best_overall_count = count;
                best_overall_idx = idx;
            }

            let ri = idx / (BUCKETS * BUCKETS);
            let gi = (idx / BUCKETS) % BUCKETS;
            let bi = idx % BUCKETS;

            // Convert to 0.0 - 1.0 scale
            let r = ri as f32 / 31.0;
            let g = gi as f32 / 31.0;
            let b = bi as f32 / 31.0;

            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let chroma = max - min;

            // Classify as black or dark/desaturated gray:
            // - max < 0.12: Very dark pixels (black / dark gray)
            // - chroma < 0.15 && max < 0.5: Grayscale/desaturated backgrounds (middle dark grays)
            let is_black_or_gray = max < 0.12 || (chroma < 0.15 && max < 0.5);

            if !is_black_or_gray {
                if count > best_non_black_count {
                    best_non_black_count = count;
                    best_non_black_idx = Some(idx);
                }
            }
        }

        let best_idx = best_non_black_idx.unwrap_or(best_overall_idx);

        let ri = best_idx / (BUCKETS * BUCKETS);
        let gi = (best_idx / BUCKETS) % BUCKETS;
        let bi = best_idx % BUCKETS;

        // Use the center of the winning bucket as the representative color.
        RgbF {
            r: (ri as f32 + 0.5) * QUANT as f32 / 255.0,
            g: (gi as f32 + 0.5) * QUANT as f32 / 255.0,
            b: (bi as f32 + 0.5) * QUANT as f32 / 255.0,
        }
    }

    impl ScreenSampler for LinuxScreenSampler {
        fn sample(&mut self, out: &mut [[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]) -> bool {
            let (img, img_w, img_h) = match capture_with_grim() {
                Some(v) => v,
                None => return false,
            };

            if img_w != self.screen_w || img_h != self.screen_h {
                self.screen_w = img_w;
                self.screen_h = img_h;
            }

            let width  = img_w as usize;
            let height = img_h as usize;

            let region_left   = (width  as f32 * self.rect_x).round() as usize;
            let region_top    = (height as f32 * self.rect_y).round() as usize;
            let region_width  = ((width  as f32) * self.rect_width).max(1.0).round() as usize;
            let region_height = ((height as f32) * self.rect_height).max(1.0).round() as usize;
            let region_end_y  = (region_top + region_height).min(height);

            let col_width = region_width as f32 / AMBIENT_WIDTH as f32;

            for x in 0..AMBIENT_WIDTH {
                let start_x = (region_left as f32 + x as f32       * col_width).round() as usize;
                let end_x   = (region_left as f32 + (x + 1) as f32 * col_width).round() as usize;
                let end_x   = end_x.max(start_x + 1).min(width);

                // Find the single most-prominent color across the entire zone column.
                let dominant = dominant_color_in_zone(&img, start_x, end_x, region_top, region_end_y);

                // Fill all AMBIENT_HEIGHT slots with this color —
                // AmbientEffect::update averages them, which is now a no-op (all equal).
                for y in 0..AMBIENT_HEIGHT {
                    out[x][y] = dominant;
                }
            }

            true
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub use linux_sampler::LinuxScreenSampler as DxgiScreenSampler;



//
// ================= AMBIENT EFFECT =================
//
// Inspired by the fast-image-resize ambient approach:
// capture → area-average to zone count → saturate → send.
// Brightness is carried naturally by the color values (dark screen = dim keyboard).
// The user's brightness slider still works as a multiplier (applied in flush_buffered).
//

pub struct AmbientEffect<S: ScreenSampler + 'static> {
    sampler: std::sync::Arc<std::sync::Mutex<S>>,
    smoothing: f32,
    last: [RgbF; AMBIENT_WIDTH],
    shared_buffer: std::sync::Arc<std::sync::Mutex<[[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]>>,
    running_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl<S: ScreenSampler + 'static> AmbientEffect<S> {
    pub fn new(sampler: S, smoothing: f32) -> Self {
        Self {
            sampler: std::sync::Arc::new(std::sync::Mutex::new(sampler)),
            smoothing,
            last: [RgbF::black(); AMBIENT_WIDTH],
            shared_buffer: std::sync::Arc::new(std::sync::Mutex::new([[RgbF::black(); AMBIENT_HEIGHT]; AMBIENT_WIDTH])),
            running_flag: None,
            thread_handle: None,
        }
    }
}

impl<S: ScreenSampler + 'static> crate::effects::Effect for AmbientEffect<S> {
    fn start(&mut self) {
        self.last = [RgbF::black(); AMBIENT_WIDTH];
        
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        self.running_flag = Some(running.clone());
        let buffer = self.shared_buffer.clone();
        let sampler = self.sampler.clone();

        let handle = std::thread::spawn(move || {
            let mut local_buf = [[RgbF::black(); AMBIENT_HEIGHT]; AMBIENT_WIDTH];
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let got_frame = if let Ok(mut sampler_guard) = sampler.lock() {
                    sampler_guard.sample(&mut local_buf)
                } else {
                    false
                };
                if got_frame {
                    if let Ok(mut guard) = buffer.lock() {
                        *guard = local_buf;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        });
        self.thread_handle = Some(handle);
    }

    fn update(&mut self, controller: &mut LedController, _time: f32, delta: f32) {
        let buffer = if let Ok(guard) = self.shared_buffer.lock() {
            *guard
        } else {
            [[RgbF::black(); AMBIENT_HEIGHT]; AMBIENT_WIDTH]
        };

        for x in 0..AMBIENT_WIDTH {
            // Average all sample rows to get the zone color
            let mut sum = RgbF::black();
            for y in 0..AMBIENT_HEIGHT {
                sum = sum.add(buffer[x][y]);
            }
            let avg = sum.scale(1.0 / AMBIENT_HEIGHT as f32);

            // Saturate for richer colors
            let target = avg.saturate(SATURATION_BOOST);

            let color = if self.smoothing > 0.01 {
                let diff = target.sub(self.last[x]);
                let len = (diff.r * diff.r + diff.g * diff.g + diff.b * diff.b).sqrt();
                let smoothed = if len > 0.0001 {
                    let step = (self.smoothing * delta).min(len);
                    self.last[x].add(diff.scale(step / len))
                } else {
                    target
                };
                self.last[x] = smoothed;
                smoothed
            } else {
                self.last[x] = target;
                target
            };

            controller.set_zone(x, color.to_color());
        }

        let _ = controller.flush_buffered();
    }

    fn stop(&mut self, controller: &mut LedController) {
        if let Some(running) = self.running_flag.take() {
            running.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        let _ = controller.clear();
    }

    fn name(&self) -> &str {
        "Ambient Screen"
    }
}

impl<S: ScreenSampler + 'static> Drop for AmbientEffect<S> {
    fn drop(&mut self) {
        if let Some(running) = self.running_flag.take() {
            running.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}