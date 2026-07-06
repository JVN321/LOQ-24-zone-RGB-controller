use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, Ordering};
use once_cell::sync::Lazy;
use std::sync::Mutex;

static INTENSITY: AtomicU32 = AtomicU32::new(0);

// Wrapper to allow cpal::Stream to be stored in a global static Mutex
// On Windows, cpal::Stream is not Send because it may contain COM pointers.
// We unsafe impl Send because we only use it to keep the stream alive.
struct SendStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendStream {}

static STREAM: Lazy<Mutex<Option<SendStream>>> = Lazy::new(|| Mutex::new(None));

pub struct AudioSampler;

impl AudioSampler {
    pub fn new() -> anyhow::Result<Self> {
        let mut stream_lock = STREAM.lock().unwrap();
        // Always drop the old stream first to ensure a fresh one is created
        *stream_lock = None;
        INTENSITY.store(0f32.to_bits(), Ordering::Relaxed);

        match Self::try_init_stream() {
            Ok(stream) => {
                *stream_lock = Some(SendStream(stream));
            }
            Err(e) => {
                eprintln!("Failed to initialize audio loopback sampler: {}", e);
                // We don't return the error, we let the sampler run in silent fallback mode
            }
        }

        Ok(Self)
    }

    fn try_init_stream() -> anyhow::Result<cpal::Stream> {
        let host = cpal::default_host();
        // On Windows, we want the default output device for loopback
        let device = host.default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No default output device found"))?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        println!("Selected audio device for loopback: {}", device_name);

        let config = device.default_output_config()?;
        println!("Audio config: {:?}", config);
        
        let err_fn = |err| eprintln!("an error occurred on audio stream: {}", err);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    Self::process_audio_f32(data);
                },
                err_fn,
                None
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    Self::process_audio_i16(data);
                },
                err_fn,
                None
            )?,
            cpal::SampleFormat::I32 => device.build_input_stream(
                &config.into(),
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    Self::process_audio_i32(data);
                },
                err_fn,
                None
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    Self::process_audio_u16(data);
                },
                err_fn,
                None
            )?,
            cpal::SampleFormat::U32 => device.build_input_stream(
                &config.into(),
                move |data: &[u32], _: &cpal::InputCallbackInfo| {
                    Self::process_audio_u32(data);
                },
                err_fn,
                None
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format {:?}", config.sample_format())),
        };

        stream.play()?;
        Ok(stream)
    }

    fn process_audio_f32(data: &[f32]) {
        if data.is_empty() { return; }
        let sum_sq: f32 = data.iter().map(|&sample| sample * sample).sum();
        let rms = (sum_sq / data.len() as f32).sqrt();
        Self::update_intensity(rms);
    }

    fn process_audio_i16(data: &[i16]) {
        if data.is_empty() { return; }
        let sum_sq: f32 = data.iter().map(|&sample| {
            let val = sample as f32 / 32767.0;
            val * val
        }).sum();
        let rms = (sum_sq / data.len() as f32).sqrt();
        Self::update_intensity(rms);
    }

    fn process_audio_i32(data: &[i32]) {
        if data.is_empty() { return; }
        let sum_sq: f32 = data.iter().map(|&sample| {
            let val = sample as f32 / 2147483647.0;
            val * val
        }).sum();
        let rms = (sum_sq / data.len() as f32).sqrt();
        Self::update_intensity(rms);
    }

    fn process_audio_u16(data: &[u16]) {
        if data.is_empty() { return; }
        let sum_sq: f32 = data.iter().map(|&sample| {
            let val = (sample as f32 - 32767.5) / 32767.5;
            val * val
        }).sum();
        let rms = (sum_sq / data.len() as f32).sqrt();
        Self::update_intensity(rms);
    }

    fn process_audio_u32(data: &[u32]) {
        if data.is_empty() { return; }
        let sum_sq: f32 = data.iter().map(|&sample| {
            let val = (sample as f32 - 2147483647.5) / 2147483647.5;
            val * val
        }).sum();
        let rms = (sum_sq / data.len() as f32).sqrt();
        Self::update_intensity(rms);
    }

    fn update_intensity(rms: f32) {
        let current_bits = INTENSITY.load(Ordering::Relaxed);
        let current = f32::from_bits(current_bits);
        let smoothed = current * 0.8 + rms * 0.2;
        INTENSITY.store(smoothed.to_bits(), Ordering::Relaxed);

        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER += 1;
            if COUNTER >= 100 {
                COUNTER = 0;
            }
        }
    }

    pub fn get_intensity(&self) -> f32 {
        f32::from_bits(INTENSITY.load(Ordering::Relaxed))
    }
}
