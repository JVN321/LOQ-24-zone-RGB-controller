#![allow(non_snake_case)]

use crate::led_driver::{LedController, Color, NUM_ZONES};

//
// ================= CONFIG =================
//

pub const AMBIENT_WIDTH: usize = NUM_ZONES;
pub const AMBIENT_HEIGHT: usize = 12;

/// Comprehensive configuration for ambient screen color extraction, dynamics, and capture.
#[derive(Clone, Copy, Debug)]
pub struct AmbientConfig {
    /// Color Extraction Algorithm:
    /// 0: Vibrant Dominant (Vivid chromatic clustering)
    /// 1: High Contrast Saturated (Aggressive black suppression + S-curve)
    /// 2: Perceptual Weighted Average (Smooth harmonic integration)
    /// 3: Dominant Hue (Color-pure modal clustering)
    /// 4: Peak Chroma (Maximum accent preservation)
    /// 5: Natural Cinema Average (Accurate linear sRGB balance)
    /// 6: Classic Saturated Average (Original pre-Linux ambient effect)
    pub algorithm: u8,

    /// Saturation multiplier (default 2.2, range 0.5 - 4.0)
    pub saturation: f32,

    /// Contrast gamma exponent (default 1.3, range 0.5 - 3.0)
    pub contrast: f32,

    /// Dark threshold below which pixels are treated as black/letterbox (default 0.08, range 0.0 - 0.4)
    pub black_cutoff: f32,

    /// Minimum brightness floor so dark zones don't completely turn off if desired (default 0.0, range 0.0 - 0.5)
    pub min_brightness: f32,

    /// Overall brightness / gain multiplier (default 1.1, range 0.5 - 2.5)
    pub brightness_boost: f32,

    /// Transition speed / agility (default 15.0, range 1.0 - 50.0)
    pub response_speed: f32,

    /// Transition Dynamics Mode:
    /// 0: Dynamic (Fast Attack on bright/colorful spikes, Smooth Decay on fades)
    /// 1: Exponential Smooth (EMA ease-in-out)
    /// 2: Instant Response (Zero-latency / 0ms for gaming)
    /// 3: Linear Step
    pub dynamic_mode: u8,

    /// Deadband noise gate to suppress subtle camera/sub-pixel jitter on static content (default 0.02, range 0.0 - 0.1)
    pub noise_threshold: f32,

    /// Capture bounding box normalized [0.0 .. 1.0]
    pub rect_x: f32,
    pub rect_y: f32,
    pub rect_width: f32,
    pub rect_height: f32,
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            algorithm: 0,
            saturation: 2.2,
            contrast: 1.3,
            black_cutoff: 0.08,
            min_brightness: 0.0,
            brightness_boost: 1.1,
            response_speed: 15.0,
            dynamic_mode: 0,
            noise_threshold: 0.02,
            rect_x: 0.0,
            rect_y: 0.0,
            rect_width: 1.0,
            rect_height: 1.0,
        }
    }
}

//
// ================= RGB FLOAT & COLOR CONVERSIONS =================
//

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RgbF {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl RgbF {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

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

    #[inline]
    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    pub fn to_color(self) -> Color {
        Color::new(
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    /// Process color through the post-processing pipeline:
    /// - For Algorithm 6 (Classic Original): Rec.709 chroma-luminance vector saturation boost & gain
    /// - For Algorithms 0-5: Black level cutoff with smooth rolloff, Gamma contrast curve, Saturation boost & floor
    pub fn process_color(self, cfg: &AmbientConfig) -> Self {
        // Algorithm 6: Original classic pre-Linux algorithm with Rec.709 chroma-luminance saturation vector expansion
        if cfg.algorithm == 6 {
            let l = self.luminance();
            let cutoff = cfg.black_cutoff.clamp(0.0, 0.95);
            if l <= cutoff || l < 0.001 {
                if cfg.min_brightness > 0.001 {
                    let mb = cfg.min_brightness.clamp(0.0, 1.0);
                    return RgbF::new(mb, mb, mb);
                }
                return RgbF::black();
            }

            // Classic Rec.709 saturation boost (pull vector away from luminance gray)
            let gray = RgbF::new(l, l, l);
            let diff = self.sub(gray);
            let sat = cfg.saturation;
            let s = gray.add(diff.scale(sat));

            let gain = cfg.brightness_boost;
            let contrast = cfg.contrast.clamp(0.2, 4.0);

            let r = if s.r > 0.0 { (s.r.powf(contrast) * gain).max(cfg.min_brightness).clamp(0.0, 1.0) } else { cfg.min_brightness.clamp(0.0, 1.0) };
            let g = if s.g > 0.0 { (s.g.powf(contrast) * gain).max(cfg.min_brightness).clamp(0.0, 1.0) } else { cfg.min_brightness.clamp(0.0, 1.0) };
            let b = if s.b > 0.0 { (s.b.powf(contrast) * gain).max(cfg.min_brightness).clamp(0.0, 1.0) } else { cfg.min_brightness.clamp(0.0, 1.0) };

            return RgbF::new(r, g, b);
        }

        let (h, s, v) = rgb_to_hsv(self.r, self.g, self.b);

        // 1. Black cutoff threshold
        let cutoff = cfg.black_cutoff.clamp(0.0, 0.95);
        let v_cut = if v <= cutoff {
            0.0
        } else if cutoff >= 0.95 {
            v
        } else {
            (v - cutoff) / (1.0 - cutoff)
        };

        // 2. Contrast gamma curve & brightness gain
        let v_contrast = if v_cut > 0.0 {
            let gamma = cfg.contrast.clamp(0.2, 4.0);
            v_cut.powf(gamma) * cfg.brightness_boost
        } else {
            0.0
        };

        // 3. Minimum brightness floor & clamping
        let v_final = if v_contrast > 0.001 {
            v_contrast.max(cfg.min_brightness).min(1.0)
        } else if cfg.min_brightness > 0.001 {
            cfg.min_brightness.clamp(0.0, 1.0)
        } else {
            0.0
        };

        // 4. Rich Saturation boost
        let s_boosted = if s > 0.001 && v_final > 0.001 {
            (s.powf(0.85) * cfg.saturation).min(1.0)
        } else {
            0.0
        };

        let (r, g, b) = hsv_to_rgb(h, s_boosted, v_final);
        Self { r, g, b }
    }
}

/// Convert normalized RGB [0.0..1.0] to HSV (H: [0..360], S: [0..1], V: [0..1])
#[inline]
pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta <= 1e-5 {
        0.0
    } else if (max - r).abs() < 1e-5 {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < 1e-5 {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max <= 1e-5 { 0.0 } else { delta / max };
    let v = max;
    (h, s, v)
}

/// Convert HSV (H: [0..360], S: [0..1], V: [0..1]) to normalized RGB [0.0..1.0]
#[inline]
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

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

    (
        (r1 + m).clamp(0.0, 1.0),
        (g1 + m).clamp(0.0, 1.0),
        (b1 + m).clamp(0.0, 1.0),
    )
}

//
// ================= COLOR EXTRACTION ALGORITHMS =================
//

/// Extracts the representative color for a zone from normalized (r, g, b) pixel samples.
pub fn extract_zone_color(samples: &[(f32, f32, f32)], algorithm: u8, black_cutoff: f32) -> RgbF {
    if samples.is_empty() {
        return RgbF::black();
    }

    match algorithm {
        // Algorithm 0: Vibrant Dominant (Vivid chromatic clustering)
        0 => extract_vibrant_dominant(samples, black_cutoff),

        // Algorithm 1: High Contrast Saturated (Aggressive black suppression + S-curve)
        1 => extract_high_contrast_saturated(samples, black_cutoff),

        // Algorithm 2: Perceptual Weighted Average (Smooth harmonic integration)
        2 => extract_perceptual_weighted_average(samples, black_cutoff),

        // Algorithm 3: Dominant Hue (Color-pure modal clustering)
        3 => extract_dominant_hue(samples, black_cutoff),

        // Algorithm 4: Peak Chroma (Maximum accent preservation)
        4 => extract_peak_chroma(samples, black_cutoff),

        // Algorithm 5: Natural Cinema Average (Accurate linear sRGB balance)
        5 => extract_natural_average(samples, black_cutoff),

        // Algorithm 6: Classic Saturated Average (Original pre-Linux ambient effect)
        6 => extract_classic_original_average(samples, black_cutoff),

        // Fallback default
        _ => extract_vibrant_dominant(samples, black_cutoff),
    }
}

/// Algorithm 0: Vibrant Dominant
/// Uses 5-bit (32x32x32) quantization with chromaticity-weighted scoring:
/// Score = count * (chroma + 0.12)^1.6 * (value + 0.05)^0.8
fn extract_vibrant_dominant(samples: &[(f32, f32, f32)], black_cutoff: f32) -> RgbF {
    const BUCKETS: usize = 32;
    let mut weights = [0.0f32; BUCKETS * BUCKETS * BUCKETS];

    let mut total_r = 0.0;
    let mut total_g = 0.0;
    let mut total_b = 0.0;
    let mut non_black_count = 0;

    for &(r, g, b) in samples {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;

        if max < black_cutoff {
            continue;
        }

        let ri = ((r * 31.0).round() as usize).min(31);
        let gi = ((g * 31.0).round() as usize).min(31);
        let bi = ((b * 31.0).round() as usize).min(31);
        let idx = ri * BUCKETS * BUCKETS + gi * BUCKETS + bi;

        // Weight gives strong preference to vivid, saturated colors over washed-out grays
        let w = (chroma + 0.12).powf(1.6) * (max + 0.05).powf(0.8);
        weights[idx] += w;

        total_r += r;
        total_g += g;
        total_b += b;
        non_black_count += 1;
    }

    let mut best_idx = None;
    let mut best_weight = 0.0f32;

    for (idx, &w) in weights.iter().enumerate() {
        if w > best_weight {
            best_weight = w;
            best_idx = Some(idx);
        }
    }

    if let Some(idx) = best_idx {
        let ri = idx / (BUCKETS * BUCKETS);
        let gi = (idx / BUCKETS) % BUCKETS;
        let bi = idx % BUCKETS;
        RgbF::new(
            (ri as f32 + 0.5) / 32.0,
            (gi as f32 + 0.5) / 32.0,
            (bi as f32 + 0.5) / 32.0,
        )
    } else if non_black_count > 0 {
        RgbF::new(
            total_r / non_black_count as f32,
            total_g / non_black_count as f32,
            total_b / non_black_count as f32,
        )
    } else {
        RgbF::black()
    }
}

/// Algorithm 1: High Contrast Saturated
/// Filters out dark/desaturated pixels and aggressively picks the highest-energy chromatic color.
fn extract_high_contrast_saturated(samples: &[(f32, f32, f32)], black_cutoff: f32) -> RgbF {
    const BUCKETS: usize = 32;
    let mut weights = [0.0f32; BUCKETS * BUCKETS * BUCKETS];
    let strict_cutoff = (black_cutoff * 1.25).min(0.8);

    let mut best_single_pixel = (0.0f32, 0.0f32, 0.0f32);
    let mut max_single_score = 0.0f32;

    for &(r, g, b) in samples {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;

        if max < strict_cutoff {
            continue;
        }

        let score = chroma.powf(2.0) * max.powf(1.5);
        if score > max_single_score {
            max_single_score = score;
            best_single_pixel = (r, g, b);
        }

        let ri = ((r * 31.0).round() as usize).min(31);
        let gi = ((g * 31.0).round() as usize).min(31);
        let bi = ((b * 31.0).round() as usize).min(31);
        let idx = ri * BUCKETS * BUCKETS + gi * BUCKETS + bi;

        weights[idx] += score + 0.01;
    }

    let mut best_idx = None;
    let mut best_weight = 0.0f32;

    for (idx, &w) in weights.iter().enumerate() {
        if w > best_weight {
            best_weight = w;
            best_idx = Some(idx);
        }
    }

    if let Some(idx) = best_idx {
        let ri = idx / (BUCKETS * BUCKETS);
        let gi = (idx / BUCKETS) % BUCKETS;
        let bi = idx % BUCKETS;
        RgbF::new(
            (ri as f32 + 0.5) / 32.0,
            (gi as f32 + 0.5) / 32.0,
            (bi as f32 + 0.5) / 32.0,
        )
    } else if max_single_score > 0.0 {
        RgbF::new(best_single_pixel.0, best_single_pixel.1, best_single_pixel.2)
    } else {
        RgbF::black()
    }
}

/// Algorithm 2: Perceptual Weighted Average
/// Integrates all samples with chromatic weighting (w = (chroma + 0.05)^1.4 * (v - cutoff)).
/// Produces an ultra-clean, harmonic average that rejects noise and gray backgrounds.
fn extract_perceptual_weighted_average(samples: &[(f32, f32, f32)], black_cutoff: f32) -> RgbF {
    let mut sum_r = 0.0f32;
    let mut sum_g = 0.0f32;
    let mut sum_b = 0.0f32;
    let mut sum_w = 0.0f32;

    for &(r, g, b) in samples {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;

        if max <= black_cutoff {
            continue;
        }

        let val_weight = (max - black_cutoff) / (1.0 - black_cutoff).max(0.01);
        let w = (chroma + 0.05).powf(1.4) * val_weight;

        sum_r += r * w;
        sum_g += g * w;
        sum_b += b * w;
        sum_w += w;
    }

    if sum_w > 1e-4 {
        RgbF::new(sum_r / sum_w, sum_g / sum_w, sum_b / sum_w)
    } else {
        RgbF::black()
    }
}

/// Algorithm 3: Dominant Hue (Color-Pure Modal Clustering)
/// Bins pixels into 24 distinct Hue bins (15 deg each) weighted by saturation and value.
/// Reconstructs the exact pure hue of the dominant chromatic cluster.
fn extract_dominant_hue(samples: &[(f32, f32, f32)], black_cutoff: f32) -> RgbF {
    const NUM_HUE_BINS: usize = 24;
    let mut bin_weights = [0.0f32; NUM_HUE_BINS];
    let mut bin_sat_accum = [0.0f32; NUM_HUE_BINS];
    let mut bin_val_accum = [0.0f32; NUM_HUE_BINS];

    let mut has_colored_pixel = false;

    for &(r, g, b) in samples {
        let (h, s, v) = rgb_to_hsv(r, g, b);
        if v < black_cutoff || s < 0.06 {
            continue;
        }

        let bin = ((h / (360.0 / NUM_HUE_BINS as f32)).floor() as usize) % NUM_HUE_BINS;
        let w = s * v;
        bin_weights[bin] += w;
        bin_sat_accum[bin] += s * w;
        bin_val_accum[bin] += v * w;
        has_colored_pixel = true;
    }

    if !has_colored_pixel {
        // Fallback to natural average if no chromatic pixels exist
        return extract_natural_average(samples, black_cutoff);
    }

    let mut best_bin = 0;
    let mut max_weight = 0.0f32;
    for (bin, &w) in bin_weights.iter().enumerate() {
        if w > max_weight {
            max_weight = w;
            best_bin = bin;
        }
    }

    if max_weight > 0.0 {
        let center_hue = (best_bin as f32 + 0.5) * (360.0 / NUM_HUE_BINS as f32);
        let avg_s = (bin_sat_accum[best_bin] / max_weight).clamp(0.0, 1.0);
        let avg_v = (bin_val_accum[best_bin] / max_weight).clamp(0.0, 1.0);
        let (r, g, b) = hsv_to_rgb(center_hue, avg_s, avg_v);
        RgbF::new(r, g, b)
    } else {
        extract_natural_average(samples, black_cutoff)
    }
}

/// Algorithm 4: Peak Chroma (Maximum Accent Preservation)
/// Scans the zone for the most saturated accent pixel (lasers, neon indicators, magic effects).
fn extract_peak_chroma(samples: &[(f32, f32, f32)], black_cutoff: f32) -> RgbF {
    let mut best_pixel = (0.0f32, 0.0f32, 0.0f32);
    let mut max_score = 0.0f32;

    for &(r, g, b) in samples {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;

        if max < black_cutoff {
            continue;
        }

        let score = chroma * (max + 0.2).sqrt();
        if score > max_score {
            max_score = score;
            best_pixel = (r, g, b);
        }
    }

    if max_score > 0.01 {
        RgbF::new(best_pixel.0, best_pixel.1, best_pixel.2)
    } else {
        extract_natural_average(samples, black_cutoff)
    }
}

/// Algorithm 5: Natural Cinema Average
/// Standard linear sRGB area average with black cutoff floor.
fn extract_natural_average(samples: &[(f32, f32, f32)], black_cutoff: f32) -> RgbF {
    let mut sum_r = 0.0f32;
    let mut sum_g = 0.0f32;
    let mut sum_b = 0.0f32;
    let mut count = 0;

    for &(r, g, b) in samples {
        let max = r.max(g).max(b);
        if max >= black_cutoff {
            sum_r += r;
            sum_g += g;
            sum_b += b;
            count += 1;
        }
    }

    if count > 0 {
        RgbF::new(sum_r / count as f32, sum_g / count as f32, sum_b / count as f32)
    } else {
        RgbF::black()
    }
}

/// Algorithm 6: Classic Saturated Average (Original Pre-Linux Windows Ambient Lighting)
/// Direct linear downsampling of zone pixels with Rec.709 chroma-luminance saturation vector expansion.
fn extract_classic_original_average(samples: &[(f32, f32, f32)], black_cutoff: f32) -> RgbF {
    let mut sum_r = 0.0f32;
    let mut sum_g = 0.0f32;
    let mut sum_b = 0.0f32;
    let mut count = 0;

    for &(r, g, b) in samples {
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        if luma >= black_cutoff {
            sum_r += r;
            sum_g += g;
            sum_b += b;
            count += 1;
        }
    }

    if count > 0 {
        RgbF::new(sum_r / count as f32, sum_g / count as f32, sum_b / count as f32)
    } else {
        RgbF::black()
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
        config: AmbientConfig,
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
                        D3D_DRIVER_TYPE_UNKNOWN,
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
                                            config: AmbientConfig::default(),
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

        pub fn set_config(&mut self, cfg: AmbientConfig) {
            self.config = cfg;
        }

        pub fn set_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
            self.config.rect_x = x.clamp(0.0, 1.0);
            self.config.rect_y = y.clamp(0.0, 1.0);
            self.config.rect_width = w.clamp(0.01, 1.0);
            self.config.rect_height = h.clamp(0.01, 1.0);
        }

        pub fn set_sample_top_fraction(&mut self, f: f32) {
            self.config.rect_y = f.clamp(0.0, 1.0);
            self.config.rect_height = (1.0 - self.config.rect_y).clamp(0.01, 1.0);
        }

        pub fn set_sample_horizontal_region(&mut self, left_frac: f32, width_frac: f32) {
            let left = left_frac.clamp(0.0, 1.0);
            let width = width_frac.clamp(0.0, 1.0);
            self.config.rect_x = left;
            self.config.rect_width = if left + width > 1.0 { 1.0 - left } else { width };
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
    }

    impl ScreenSampler for DxgiScreenSampler {
        fn sample(&mut self, out: &mut [[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]) -> bool {
            unsafe {
                let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut resource = None;

                if let Err(e) = self.duplication.AcquireNextFrame(0, &mut frame_info, &mut resource) {
                    if e.code() != DXGI_ERROR_WAIT_TIMEOUT {
                        let _ = self.recreate_duplication();
                    }
                    return false;
                }

                let has_update = frame_info.LastPresentTime != 0 || frame_info.AccumulatedFrames > 0;

                let texture: ID3D11Texture2D = resource.unwrap().cast().unwrap();
                self.context.CopyResource(&self.staging, &texture);

                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                if self.context.Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).is_ok() {
                    let data = mapped.pData as *const u8;
                    let pitch = mapped.RowPitch as usize;

                    let region_left = (self.width as f32 * self.config.rect_x) as usize;
                    let region_top = (self.height as f32 * self.config.rect_y) as usize;
                    let region_width = ((self.width as f32) * self.config.rect_width).max(1.0) as usize;
                    let region_height = ((self.height as f32) * self.config.rect_height).max(1.0) as usize;
                    let region_end_y = (region_top + region_height).min(self.height as usize);

                    let col_width = region_width as f32 / AMBIENT_WIDTH as f32;
                    let mut zone_samples = Vec::with_capacity(256);

                    for x in 0..AMBIENT_WIDTH {
                        let start_x = (region_left as f32 + x as f32 * col_width) as usize;
                        let end_x = (region_left as f32 + (x + 1) as f32 * col_width) as usize;
                        let end_x = end_x.max(start_x + 1).min(self.width as usize);

                        zone_samples.clear();
                        let band_w = (end_x - start_x).max(1);
                        let band_h = (region_end_y - region_top).max(1);
                        let x_step = (band_w / 16).max(1);
                        let y_step = (band_h / 16).max(1);

                        for sy in (region_top..region_end_y).step_by(y_step) {
                            let sy_clamped = sy.min(self.height as usize - 1);
                            for sx in (start_x..end_x).step_by(x_step) {
                                let sx_clamped = sx.min(self.width as usize - 1);
                                let p = data.add(sy_clamped * pitch + sx_clamped * 4);
                                // BGRA format
                                let b = *p.add(0) as f32 / 255.0;
                                let g = *p.add(1) as f32 / 255.0;
                                let r = *p.add(2) as f32 / 255.0;
                                zone_samples.push((r, g, b));
                            }
                        }

                        let zone_color = extract_zone_color(&zone_samples, self.config.algorithm, self.config.black_cutoff);

                        for y in 0..AMBIENT_HEIGHT {
                            out[x][y] = zone_color;
                        }
                    }

                    self.context.Unmap(&self.staging, 0);
                }

                let _ = self.duplication.ReleaseFrame();
                has_update
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use dxgi::DxgiScreenSampler;

//
// ================= LINUX SCREEN SAMPLER =================
//

#[cfg(not(target_os = "windows"))]
mod linux_sampler {
    use super::*;
    use std::os::unix::fs::FileTypeExt;

    /// Fast Netpbm P6 PPM decoder for zero-latency screen capture.
    fn parse_ppm_rgb(data: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
        if data.len() < 10 || data[0] != b'P' || data[1] != b'6' {
            return None;
        }
        let mut pos = 2;

        let skip_ws_and_comments = |data: &[u8], mut p: usize| -> usize {
            while p < data.len() {
                let b = data[p];
                if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                    p += 1;
                } else if b == b'#' {
                    while p < data.len() && data[p] != b'\n' {
                        p += 1;
                    }
                } else {
                    break;
                }
            }
            p
        };

        pos = skip_ws_and_comments(data, pos);
        let start = pos;
        while pos < data.len() && data[pos].is_ascii_digit() {
            pos += 1;
        }
        let width: u32 = std::str::from_utf8(&data[start..pos]).ok()?.parse().ok()?;

        pos = skip_ws_and_comments(data, pos);
        let start = pos;
        while pos < data.len() && data[pos].is_ascii_digit() {
            pos += 1;
        }
        let height: u32 = std::str::from_utf8(&data[start..pos]).ok()?.parse().ok()?;

        pos = skip_ws_and_comments(data, pos);
        let start = pos;
        while pos < data.len() && data[pos].is_ascii_digit() {
            pos += 1;
        }
        let _maxval: u32 = std::str::from_utf8(&data[start..pos]).ok()?.parse().ok()?;

        if pos < data.len() && (data[pos] == b' ' || data[pos] == b'\n' || data[pos] == b'\r') {
            pos += 1;
        }

        let expected_len = (width as usize) * (height as usize) * 3;
        if data.len() < pos + expected_len {
            return None;
        }

        let rgb_bytes = data[pos..pos + expected_len].to_vec();
        Some((rgb_bytes, width, height))
    }

    /// Capture screen using `grim -t ppm -` (fastest) with PNG fallback.
    fn capture_screen_raw() -> Option<(Vec<u8>, u32, u32)> {
        let mut cmd = std::process::Command::new("grim");
        cmd.args(["-t", "ppm", "-"]);

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

        if std::env::var("WAYLAND_DISPLAY").map(|v| v.trim().is_empty()).unwrap_or(true) {
            if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
                let mut sockets: Vec<String> = entries
                    .flatten()
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        let is_socket = e.file_type().map(|ft| ft.is_socket()).unwrap_or(false);
                        if is_socket && name.starts_with("wayland-") && !name.ends_with(".lock") {
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

        let output = cmd.output().ok()?;
        if !output.status.success() || output.stdout.is_empty() {
            return None;
        }

        // 1. Try instantaneous PPM parsing (<1ms)
        if let Some(ppm_res) = parse_ppm_rgb(&output.stdout) {
            return Some(ppm_res);
        }

        // 2. Fallback to image decoder if format was PNG
        if let Ok(img) = xcap::image::load_from_memory(&output.stdout) {
            let rgb = img.to_rgb8();
            let w = rgb.width();
            let h = rgb.height();
            return Some((rgb.into_raw(), w, h));
        }

        None
    }

    pub struct LinuxScreenSampler {
        config: AmbientConfig,
        screen_w: u32,
        screen_h: u32,
    }

    impl LinuxScreenSampler {
        pub fn new() -> anyhow::Result<Self> {
            let (screen_w, screen_h) = capture_screen_raw()
                .map(|(_, w, h)| (w, h))
                .unwrap_or((1920, 1080));

            Ok(Self {
                config: AmbientConfig::default(),
                screen_w,
                screen_h,
            })
        }

        pub fn set_config(&mut self, cfg: AmbientConfig) {
            self.config = cfg;
        }

        pub fn set_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
            self.config.rect_x = x.clamp(0.0, 1.0);
            self.config.rect_y = y.clamp(0.0, 1.0);
            self.config.rect_width = w.clamp(0.01, 1.0);
            self.config.rect_height = h.clamp(0.01, 1.0);
        }

        pub fn set_sample_top_fraction(&mut self, f: f32) {
            self.config.rect_y = f.clamp(0.0, 1.0);
            self.config.rect_height = (1.0 - self.config.rect_y).clamp(0.01, 1.0);
        }

        pub fn set_sample_horizontal_region(&mut self, left_frac: f32, width_frac: f32) {
            let left = left_frac.clamp(0.0, 1.0);
            let width = width_frac.clamp(0.0, 1.0);
            self.config.rect_x = left;
            self.config.rect_width = if left + width > 1.0 { 1.0 - left } else { width };
        }
    }

    impl ScreenSampler for LinuxScreenSampler {
        fn sample(&mut self, out: &mut [[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]) -> bool {
            let (rgb_bytes, img_w, img_h) = match capture_screen_raw() {
                Some(v) => v,
                None => return false,
            };

            if img_w != self.screen_w || img_h != self.screen_h {
                self.screen_w = img_w;
                self.screen_h = img_h;
            }

            let width = img_w as usize;
            let height = img_h as usize;

            let region_left   = (width  as f32 * self.config.rect_x).round() as usize;
            let region_top    = (height as f32 * self.config.rect_y).round() as usize;
            let region_width  = ((width  as f32) * self.config.rect_width).max(1.0).round() as usize;
            let region_height = ((height as f32) * self.config.rect_height).max(1.0).round() as usize;
            let region_end_y  = (region_top + region_height).min(height);

            let col_width = region_width as f32 / AMBIENT_WIDTH as f32;
            let mut zone_samples = Vec::with_capacity(256);

            for x in 0..AMBIENT_WIDTH {
                let start_x = (region_left as f32 + x as f32       * col_width).round() as usize;
                let end_x   = (region_left as f32 + (x + 1) as f32 * col_width).round() as usize;
                let end_x   = end_x.max(start_x + 1).min(width);

                zone_samples.clear();
                let band_w = (end_x - start_x).max(1);
                let band_h = (region_end_y - region_top).max(1);
                let x_step = (band_w / 16).max(1);
                let y_step = (band_h / 16).max(1);

                for sy in (region_top..region_end_y).step_by(y_step) {
                    let sy_clamped = sy.min(height - 1);
                    let row_offset = sy_clamped * width * 3;
                    for sx in (start_x..end_x).step_by(x_step) {
                        let sx_clamped = sx.min(width - 1);
                        let p_offset = row_offset + sx_clamped * 3;
                        if p_offset + 2 < rgb_bytes.len() {
                            let r = rgb_bytes[p_offset] as f32 / 255.0;
                            let g = rgb_bytes[p_offset + 1] as f32 / 255.0;
                            let b = rgb_bytes[p_offset + 2] as f32 / 255.0;
                            zone_samples.push((r, g, b));
                        }
                    }
                }

                let zone_color = extract_zone_color(&zone_samples, self.config.algorithm, self.config.black_cutoff);

                for y in 0..AMBIENT_HEIGHT {
                    out[x][y] = zone_color;
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

pub struct AmbientEffect<S: ScreenSampler + 'static> {
    sampler: std::sync::Arc<std::sync::Mutex<S>>,
    config: AmbientConfig,
    last: [RgbF; AMBIENT_WIDTH],
    shared_buffer: std::sync::Arc<std::sync::Mutex<[[RgbF; AMBIENT_HEIGHT]; AMBIENT_WIDTH]>>,
    running_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl<S: ScreenSampler + 'static> AmbientEffect<S> {
    pub fn new(sampler: S, config: AmbientConfig) -> Self {
        Self {
            sampler: std::sync::Arc::new(std::sync::Mutex::new(sampler)),
            config,
            last: [RgbF::black(); AMBIENT_WIDTH],
            shared_buffer: std::sync::Arc::new(std::sync::Mutex::new([[RgbF::black(); AMBIENT_HEIGHT]; AMBIENT_WIDTH])),
            running_flag: None,
            thread_handle: None,
        }
    }

    /// Legacy convenience constructor
    pub fn new_with_smoothing(sampler: S, smoothing: f32) -> Self {
        let mut cfg = AmbientConfig::default();
        cfg.response_speed = smoothing * 5.0;
        Self::new(sampler, cfg)
    }

    pub fn set_config(&mut self, config: AmbientConfig) {
        self.config = config;
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
                std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 fps capture rate
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
            // Average rows in column (already uniform or sampled)
            let mut sum = RgbF::black();
            for y in 0..AMBIENT_HEIGHT {
                sum = sum.add(buffer[x][y]);
            }
            let raw_zone = sum.scale(1.0 / AMBIENT_HEIGHT as f32);

            // Apply color pipeline (contrast, saturation boost, black cutoff, min brightness floor)
            let target = raw_zone.process_color(&self.config);
            let last = self.last[x];

            let diff = target.sub(last);
            let dist = (diff.r * diff.r + diff.g * diff.g + diff.b * diff.b).sqrt();

            // Temporal dynamics & responsive transition
            let new_color = if dist < self.config.noise_threshold {
                // Jitter reduction deadband: suppress subtle sub-pixel noise
                last
            } else {
                match self.config.dynamic_mode {
                    // Mode 0: Dynamic (Fast Attack on bright/colorful changes, Smooth Decay on fades)
                    0 => {
                        let target_luma = 0.299 * target.r + 0.587 * target.g + 0.114 * target.b;
                        let last_luma = 0.299 * last.r + 0.587 * last.g + 0.114 * last.b;
                        let is_attack = target_luma > last_luma || dist > 0.35;
                        let speed_mult = if is_attack { 2.5 } else { 0.75 };
                        let effective_speed = (self.config.response_speed * speed_mult).clamp(0.5, 100.0);
                        let alpha = (1.0 - (-effective_speed * delta).exp()).clamp(0.0, 1.0);
                        last.add(diff.scale(alpha))
                    }
                    // Mode 1: Exponential Smooth (EMA ease-in-out)
                    1 => {
                        let alpha = (1.0 - (-self.config.response_speed * delta).exp()).clamp(0.0, 1.0);
                        last.add(diff.scale(alpha))
                    }
                    // Mode 2: Instant Zero-Latency (0ms for competitive gaming)
                    2 => target,
                    // Mode 3 or default: Linear Step
                    _ => {
                        let step = (self.config.response_speed * delta).min(dist);
                        if dist > 0.0001 {
                            last.add(diff.scale(step / dist))
                        } else {
                            target
                        }
                    }
                }
            };

            self.last[x] = new_color;
            controller.set_zone(x, new_color.to_color());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_classic_original_average() {
        let samples = vec![
            (1.0, 0.0, 0.0), // Red (luma = 0.2126)
            (0.0, 1.0, 0.0), // Green (luma = 0.7152)
            (0.0, 0.0, 0.0), // Black (luma = 0.0)
        ];

        // With black cutoff 0.05, the black pixel is excluded, averaging Red and Green -> (0.5, 0.5, 0.0)
        let color = extract_zone_color(&samples, 6, 0.05);
        assert!((color.r - 0.5).abs() < 1e-4);
        assert!((color.g - 0.5).abs() < 1e-4);
        assert_eq!(color.b, 0.0);
    }

    #[test]
    fn test_process_color_algorithm_6() {
        let mut cfg = AmbientConfig::default();
        cfg.algorithm = 6;
        cfg.saturation = 2.0;
        cfg.contrast = 1.0;
        cfg.brightness_boost = 1.0;
        cfg.black_cutoff = 0.0;
        cfg.min_brightness = 0.0;

        let input = RgbF::new(0.6, 0.4, 0.2);
        let luma: f32 = 0.2126 * 0.6 + 0.7152 * 0.4 + 0.0722 * 0.2;
        let processed = input.process_color(&cfg);

        // Expected: luma + (channel - luma) * 2.0
        let exp_r = (luma + (0.6f32 - luma) * 2.0f32).clamp(0.0f32, 1.0f32);
        let exp_g = (luma + (0.4f32 - luma) * 2.0f32).clamp(0.0f32, 1.0f32);
        let exp_b = (luma + (0.2f32 - luma) * 2.0f32).clamp(0.0f32, 1.0f32);

        assert!((processed.r - exp_r).abs() < 1e-4);
        assert!((processed.g - exp_g).abs() < 1e-4);
        assert!((processed.b - exp_b).abs() < 1e-4);
    }
}