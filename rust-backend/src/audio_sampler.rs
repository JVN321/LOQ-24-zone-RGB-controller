/// Audio sampler — captures system audio, microphone, or both, and computes RMS intensity.
///
/// On Windows: uses WASAPI loopback/capture.
/// On Linux:   uses CPAL (microphone) and/or spawns `parec` (system loopback).

use std::sync::atomic::{AtomicU32, Ordering};

static INTENSITY: AtomicU32 = AtomicU32::new(0);

// ============================================================
// Windows (WASAPI) implementation
// ============================================================

#[cfg(target_os = "windows")]
mod wasapi_impl {
    use super::*;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;
    use std::sync::atomic::AtomicBool;

    struct CaptureHandle {
        stop_flag: Arc<AtomicBool>,
        _thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Drop for CaptureHandle {
        fn drop(&mut self) {
            self.stop_flag.store(true, Ordering::SeqCst);
            if let Some(handle) = self._thread.take() {
                let _ = handle.join();
            }
        }
    }

    pub struct AudioSampler {
        _handles: Vec<CaptureHandle>,
    }

    impl AudioSampler {
        pub fn new(source: f32) -> anyhow::Result<Self> {
            INTENSITY.store(0f32.to_bits(), Ordering::Relaxed);
            let mut handles = Vec::new();

            // 0.0 = System Audio, 2.0 = Both
            if source == 0.0 || source == 2.0 {
                let stop_flag = Arc::new(AtomicBool::new(false));
                let stop_clone = stop_flag.clone();
                let thread = std::thread::spawn(move || {
                    unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
                    if let Err(e) = Self::capture_loop(&stop_clone, eRender, AUDCLNT_STREAMFLAGS_LOOPBACK) {
                        eprintln!("[AudioSampler] WASAPI system capture error: {}", e);
                    }
                    unsafe { CoUninitialize(); }
                });
                handles.push(CaptureHandle { stop_flag, _thread: Some(thread) });
            }

            // 1.0 = Microphone, 2.0 = Both
            if source == 1.0 || source == 2.0 {
                let stop_flag = Arc::new(AtomicBool::new(false));
                let stop_clone = stop_flag.clone();
                let thread = std::thread::spawn(move || {
                    unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
                    if let Err(e) = Self::capture_loop(&stop_clone, eCapture, 0) {
                        eprintln!("[AudioSampler] WASAPI mic capture error: {}", e);
                    }
                    unsafe { CoUninitialize(); }
                });
                handles.push(CaptureHandle { stop_flag, _thread: Some(thread) });
            }

            Ok(Self { _handles: handles })
        }

        fn capture_loop(
            stop_flag: &AtomicBool,
            data_flow: EDataFlow,
            flags: u32,
        ) -> anyhow::Result<()> {
            unsafe {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let device = enumerator.GetDefaultAudioEndpoint(data_flow, eConsole)?;
                let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
                let pwfx = audio_client.GetMixFormat()?;
                let wfx = &*pwfx;
                let channels = wfx.nChannels as usize;
                let bits_per_sample = wfx.wBitsPerSample;
                let is_float = if wfx.wFormatTag == 0xFFFE {
                    let ext = &*(pwfx as *const WAVEFORMATEX as *const WAVEFORMATEXTENSIBLE);
                    let sub_format = std::ptr::addr_of!(ext.SubFormat).read_unaligned();
                    sub_format == windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00AA00389B71)
                } else { wfx.wFormatTag == 3 };

                audio_client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 2_000_000, 0, pwfx, None)?;
                let capture_client: IAudioCaptureClient = audio_client.GetService()?;
                audio_client.Start()?;

                while !stop_flag.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    loop {
                        let next_packet_size = match capture_client.GetNextPacketSize() {
                            Ok(size) => size, Err(_) => break,
                        };
                        if next_packet_size == 0 { break; }
                        let mut buffer_ptr = std::ptr::null_mut();
                        let mut num_frames = 0u32;
                        let mut flags_out = 0u32;
                        if capture_client.GetBuffer(&mut buffer_ptr, &mut num_frames, &mut flags_out, None, None).is_err() { break; }
                        let total_samples = num_frames as usize * channels;
                        if flags_out & 0x2 == 0 && total_samples > 0 {
                            let rms = if is_float && bits_per_sample == 32 {
                                let samples = std::slice::from_raw_parts(buffer_ptr as *const f32, total_samples);
                                let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
                                (sum_sq / total_samples as f32).sqrt()
                            } else if !is_float && bits_per_sample == 16 {
                                let samples = std::slice::from_raw_parts(buffer_ptr as *const i16, total_samples);
                                let sum_sq: f32 = samples.iter().map(|&s| { let v = s as f32 / 32767.0; v * v }).sum();
                                (sum_sq / total_samples as f32).sqrt()
                            } else { 0.0 };
                            Self::update_intensity(rms);
                        }
                        let _ = capture_client.ReleaseBuffer(num_frames);
                    }
                }
                let _ = audio_client.Stop();
                CoTaskMemFree(Some(pwfx as *const _ as *const _));
            }
            Ok(())
        }

        fn update_intensity(rms: f32) {
            let current = f32::from_bits(INTENSITY.load(Ordering::Relaxed));
            let smoothed = current * 0.4 + rms * 0.6;
            INTENSITY.store(smoothed.to_bits(), Ordering::Relaxed);
        }

        pub fn get_intensity(&self) -> f32 {
            f32::from_bits(INTENSITY.load(Ordering::Relaxed))
        }
    }
}

// ============================================================
// Linux (CPAL / Subprocess) implementation
// ============================================================

#[cfg(not(target_os = "windows"))]
mod cpal_impl {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::process::{Command, Stdio, Child};
    use std::io::Read;

    pub struct SendStream(pub cpal::Stream);
    unsafe impl Send for SendStream {}
    unsafe impl Sync for SendStream {}

    pub struct AudioSampler {
        _stream: Option<SendStream>,
        child: Option<Child>,
    }

    impl AudioSampler {
        pub fn new(source: f32) -> anyhow::Result<Self> {
            INTENSITY.store(0f32.to_bits(), Ordering::Relaxed);

            let mut stream = None;
            let mut child = None;

            // 1. Microphone or Both -> CPAL default input device
            if source == 1.0 || source == 2.0 {
                let host = cpal::default_host();
                if let Some(device) = host.default_input_device() {
                    if let Ok(config) = device.default_input_config() {
                        let channels = config.channels() as usize;
                        let s = match config.sample_format() {
                            cpal::SampleFormat::F32 => {
                                device.build_input_stream(
                                    &config.into(),
                                    move |data: &[f32], _| Self::process_f32(data, channels),
                                    |e| eprintln!("[AudioSampler] mic stream error: {}", e),
                                    None,
                                ).map(SendStream).map_err(anyhow::Error::from)
                            }
                            cpal::SampleFormat::I16 => {
                                device.build_input_stream(
                                    &config.into(),
                                    move |data: &[i16], _| Self::process_i16(data, channels),
                                    |e| eprintln!("[AudioSampler] mic stream error: {}", e),
                                    None,
                                ).map(SendStream).map_err(anyhow::Error::from)
                            }
                            _ => Err(anyhow::anyhow!("Unsupported format")),
                        };
                        if let Ok(s) = s {
                            if s.0.play().is_ok() {
                                stream = Some(s);
                            }
                        }
                    }
                }
            }

            // 2. System Audio or Both -> Spawn parec capturing from monitor
            if source == 0.0 || source == 2.0 {
                let monitor = Self::get_default_sink_monitor();
                match Command::new("parec")
                    .args(&[
                        "--device", &monitor,
                        "--format", "s16le",
                        "--channels", "1",
                        "--rate", "44100",
                        "--latency-msec=20",
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(mut c) => {
                        if let Some(mut stdout) = c.stdout.take() {
                            child = Some(c);
                            std::thread::spawn(move || {
                                let mut buf = [0u8; 512];
                                loop {
                                    match stdout.read(&mut buf) {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            let samples = n / 2;
                                            if samples > 0 {
                                                let mut sum_sq = 0.0;
                                                for i in 0..samples {
                                                    let val = i16::from_le_bytes([buf[i*2], buf[i*2+1]]) as f32 / 32767.0;
                                                    sum_sq += val * val;
                                                }
                                                let rms = (sum_sq / samples as f32).sqrt();
                                                Self::update_intensity(rms);
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        }
                    }
                    Err(e) => {
                        eprintln!("[AudioSampler] Failed to start parec for system audio: {}", e);
                    }
                }
            }

            Ok(Self { _stream: stream, child })
        }

        fn get_default_sink_monitor() -> String {
            if let Ok(output) = Command::new("pactl").arg("get-default-sink").output() {
                let sink = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !sink.is_empty() {
                    return format!("{}.monitor", sink);
                }
            }
            "auto_null.monitor".to_string()
        }

        fn process_f32(data: &[f32], _channels: usize) {
            if data.is_empty() { return; }
            let sum_sq: f32 = data.iter().map(|&s| s * s).sum();
            let rms = (sum_sq / data.len() as f32).sqrt();
            Self::update_intensity(rms);
        }

        fn process_i16(data: &[i16], _channels: usize) {
            if data.is_empty() { return; }
            let sum_sq: f32 = data.iter()
                .map(|&s| { let v = s as f32 / 32767.0; v * v })
                .sum();
            let rms = (sum_sq / data.len() as f32).sqrt();
            Self::update_intensity(rms);
        }

        fn update_intensity(rms: f32) {
            let current = f32::from_bits(INTENSITY.load(Ordering::Relaxed));
            let smoothed = current * 0.4 + rms * 0.6;
            INTENSITY.store(smoothed.to_bits(), Ordering::Relaxed);
        }

        pub fn get_intensity(&self) -> f32 {
            f32::from_bits(INTENSITY.load(Ordering::Relaxed))
        }
    }

    impl Drop for AudioSampler {
        fn drop(&mut self) {
            if let Some(mut c) = self.child.take() {
                let _ = c.kill();
            }
        }
    }
}

// ============================================================
// Re-export the platform-appropriate AudioSampler
// ============================================================

#[cfg(target_os = "windows")]
pub use wasapi_impl::AudioSampler;

#[cfg(not(target_os = "windows"))]
pub use cpal_impl::AudioSampler;
