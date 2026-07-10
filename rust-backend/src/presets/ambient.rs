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
        let s_boosted = if s > 0.005 {
            (s.powf(0.25) * 1.95).min(1.0)
        } else {
            s
        };

        // Boost Value (brightness) slightly for dimmer colors to pop
        let v_boosted = if v > 0.01 {
            (v.powf(0.75) * 1.2).min(1.0)
        } else {
            v
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
        // sampling region expressed as fractions [0.0..1.0]
        sample_top_frac: f32,    // fraction from top where sampling region starts (was 0.85)
        sample_left_frac: f32,   // fraction from left where horizontal region starts
        sample_width_frac: f32,  // fraction of total width to sample
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
                                            sample_top_frac: 0.0,
                                            sample_left_frac: 0.0,
                                            sample_width_frac: 1.0,
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

        /// Set the vertical start as a fraction [0.0..1.0] from the top of the screen
        pub fn set_sample_top_fraction(&mut self, f: f32) {
            self.sample_top_frac = f.clamp(0.0, 1.0);
        }

        /// Set the horizontal sampling region using left offset and width (fractions)
        pub fn set_sample_horizontal_region(&mut self, left_frac: f32, width_frac: f32) {
            let left = left_frac.clamp(0.0, 1.0);
            let width = width_frac.clamp(0.0, 1.0);
            self.sample_left_frac = left;
            // ensure region stays within bounds
            self.sample_width_frac = if left + width > 1.0 { 1.0 - left } else { width };
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
                    let y_start = (self.height as f32 * self.sample_top_frac) as usize;
                    let region_left = (self.width as f32 * self.sample_left_frac) as usize;
                    let region_width = ((self.width as f32) * self.sample_width_frac).max(1.0) as usize;

                    for x in 0..AMBIENT_WIDTH {
                        for y in 0..AMBIENT_HEIGHT {
                            // map zone index to region pixel (clamped)
                            let sx_rel = x * region_width / AMBIENT_WIDTH;
                            let sx = (region_left + sx_rel).min(self.width as usize - 1);
                            let sy = y_start + y * (self.height as usize - y_start) / AMBIENT_HEIGHT;
                            let p = data.add(sy * pitch + sx * 4);

                            out[x][y] = RgbF {
                                b: *p.add(0) as f32 / 255.0,
                                g: *p.add(1) as f32 / 255.0,
                                r: *p.add(2) as f32 / 255.0,
                            };
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

    pub struct LinuxScreenSampler {
        monitor: Option<xcap::Monitor>,
        sample_top_frac: f32,
        sample_left_frac: f32,
        sample_width_frac: f32,
    }

    impl LinuxScreenSampler {
        pub fn new() -> anyhow::Result<Self> {
            let monitors = xcap::Monitor::all().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let monitor = monitors.into_iter().next();
            Ok(Self {
                monitor,
                sample_top_frac: 0.0,
                sample_left_frac: 0.0,
                sample_width_frac: 1.0,
            })
        }

        pub fn set_sample_top_fraction(&mut self, f: f32) {
            self.sample_top_frac = f.clamp(0.0, 1.0);
        }

        pub fn set_sample_horizontal_region(&mut self, left_frac: f32, width_frac: f32) {
            let left = left_frac.clamp(0.0, 1.0);
            let width = width_frac.clamp(0.0, 1.0);
            self.sample_left_frac = left;
            self.sample_width_frac = if left + width > 1.0 { 1.0 - left } else { width };
        }
    }

    impl ScreenSampler for LinuxScreenSampler {
        fn sample(&mut self, out: &mut [[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]) -> bool {
            let monitor = match &self.monitor {
                Some(m) => m,
                None => return false,
            };
            
            let img = match monitor.capture_image() {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("⚠️  Ambient capture failed: {:?}", e);
                    return false;
                }
            };
            
            let width = img.width() as usize;
            let height = img.height() as usize;
            if width == 0 || height == 0 {
                return false;
            }
            
            let y_start = (height as f32 * self.sample_top_frac) as usize;
            let region_left = (width as f32 * self.sample_left_frac) as usize;
            let region_width = ((width as f32) * self.sample_width_frac).max(1.0) as usize;
            
            for x in 0..AMBIENT_WIDTH {
                for y in 0..AMBIENT_HEIGHT {
                    let sx_rel = x * region_width / AMBIENT_WIDTH;
                    let sx = (region_left + sx_rel).min(width - 1);
                    let sy = y_start + y * (height - y_start) / AMBIENT_HEIGHT;
                    let sy = sy.min(height - 1);
                    
                    let pixel = img.get_pixel(sx as u32, sy as u32);
                    out[x][y] = RgbF {
                        r: pixel[0] as f32 / 255.0,
                        g: pixel[1] as f32 / 255.0,
                        b: pixel[2] as f32 / 255.0,
                    };
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