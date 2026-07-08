use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use once_cell::sync::Lazy;

use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

static INTENSITY: AtomicU32 = AtomicU32::new(0);

/// Handle wrapper for the loopback capture thread.
/// The thread runs until this is dropped (via the stop flag).
struct LoopbackHandle {
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    _thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for LoopbackHandle {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self._thread.take() {
            let _ = handle.join();
        }
    }
}

// Safety: the thread handle and atomic flag are inherently Send-safe.
unsafe impl Send for LoopbackHandle {}

static LOOPBACK: Lazy<std::sync::Mutex<Option<LoopbackHandle>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

pub struct AudioSampler;

impl AudioSampler {
    pub fn new() -> anyhow::Result<Self> {
        let mut lock = LOOPBACK.lock().unwrap();
        // Drop any existing capture thread
        *lock = None;
        INTENSITY.store(0f32.to_bits(), Ordering::Relaxed);

        let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        let thread = std::thread::spawn(move || {
            // Each thread needs its own COM initialization (MTA for audio)
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }

            if let Err(e) = Self::capture_loop(&stop_clone) {
                eprintln!("WASAPI loopback capture error: {}", e);
            }

            unsafe {
                CoUninitialize();
            }
        });

        *lock = Some(LoopbackHandle {
            stop_flag,
            _thread: Some(thread),
        });

        Ok(Self)
    }

    /// The main WASAPI loopback capture loop.
    /// Captures audio from the current default render (output) endpoint,
    /// which works with speakers, wired headphones, Bluetooth, etc.
    fn capture_loop(stop_flag: &std::sync::atomic::AtomicBool) -> anyhow::Result<()> {
        unsafe {
            // Get the default audio render endpoint (whatever Windows is currently outputting to)
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;

            // Activate IAudioClient on the render device
            let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

            // Get the mix format (the format Windows is using for the output)
            let pwfx = audio_client.GetMixFormat()?;
            let wfx = &*pwfx;

            let channels = wfx.nChannels as usize;
            let bits_per_sample = wfx.wBitsPerSample;

            // Determine if the format is IEEE float
            // WAVE_FORMAT_EXTENSIBLE = 0xFFFE, WAVE_FORMAT_IEEE_FLOAT = 0x0003
            let is_float = if wfx.wFormatTag == 0xFFFE {
                // Cast to WAVEFORMATEXTENSIBLE to check SubFormat GUID
                let ext = &*(pwfx as *const WAVEFORMATEX as *const WAVEFORMATEXTENSIBLE);
                // Safe read of unaligned field in packed struct
                let sub_format = std::ptr::addr_of!(ext.SubFormat).read_unaligned();
                // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT = {00000003-0000-0010-8000-00AA00389B71}
                sub_format
                    == windows::core::GUID::from_u128(
                        0x00000003_0000_0010_8000_00AA00389B71,
                    )
            } else {
                wfx.wFormatTag == 3
            };

            // Initialize in shared loopback mode
            // AUDCLNT_STREAMFLAGS_LOOPBACK = 0x00020000
            let buffer_duration: i64 = 2_000_000; // 200ms in 100ns units
            audio_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                buffer_duration,
                0,
                pwfx,
                None,
            )?;

            let capture_client: IAudioCaptureClient = audio_client.GetService()?;
            audio_client.Start()?;

            // Capture loop: poll for packets every ~20ms
            while !stop_flag.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(20));

                loop {
                    let next_packet_size = match capture_client.GetNextPacketSize() {
                        Ok(size) => size,
                        Err(_) => break,
                    };

                    if next_packet_size == 0 {
                        break;
                    }

                    let mut buffer_ptr = std::ptr::null_mut();
                    let mut num_frames = 0u32;
                    let mut flags = 0u32;

                    if capture_client
                        .GetBuffer(
                            &mut buffer_ptr,
                            &mut num_frames,
                            &mut flags,
                            None,
                            None,
                        )
                        .is_err()
                    {
                        break;
                    }

                    let total_samples = num_frames as usize * channels;

                    // AUDCLNT_BUFFERFLAGS_SILENT = 0x2
                    if flags & 0x2 == 0 && total_samples > 0 {
                        let rms = if is_float && bits_per_sample == 32 {
                            let samples = std::slice::from_raw_parts(
                                buffer_ptr as *const f32,
                                total_samples,
                            );
                            let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
                            (sum_sq / total_samples as f32).sqrt()
                        } else if !is_float && bits_per_sample == 16 {
                            let samples = std::slice::from_raw_parts(
                                buffer_ptr as *const i16,
                                total_samples,
                            );
                            let sum_sq: f32 = samples
                                .iter()
                                .map(|&s| {
                                    let v = s as f32 / 32767.0;
                                    v * v
                                })
                                .sum();
                            (sum_sq / total_samples as f32).sqrt()
                        } else {
                            0.0
                        };

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
        let current_bits = INTENSITY.load(Ordering::Relaxed);
        let current = f32::from_bits(current_bits);
        let smoothed = current * 0.8 + rms * 0.2;
        INTENSITY.store(smoothed.to_bits(), Ordering::Relaxed);
    }

    pub fn get_intensity(&self) -> f32 {
        f32::from_bits(INTENSITY.load(Ordering::Relaxed))
    }
}
