use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream};
use parking_lot::Mutex;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const TARGET_SAMPLE_RATE: u32 = 16000;

pub struct AudioRecorder {
    is_recording: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    latest_rms: Arc<Mutex<f32>>,
    device_sample_rate: Arc<Mutex<u32>>,
    preroll_buffer: Arc<Mutex<VecDeque<f32>>>,
    active_device_name: Arc<Mutex<String>>,
    #[allow(clippy::arc_with_non_send_sync)]
    stream: Arc<Mutex<Option<Stream>>>,
}

unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new() -> Self {
        let is_recording = Arc::new(AtomicBool::new(false));
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(TARGET_SAMPLE_RATE as usize * 30)));
        let latest_rms = Arc::new(Mutex::new(0.0));
        let device_sample_rate = Arc::new(Mutex::new(TARGET_SAMPLE_RATE));
        let preroll_buffer = Arc::new(Mutex::new(VecDeque::<f32>::new()));
        let active_device_name = Arc::new(Mutex::new("Default Microphone".to_string()));

        let host = cpal::default_host();
        let input_devices: Vec<Device> = host.input_devices().map(|d| d.collect()).unwrap_or_default();

        // Print all available input devices
        println!("==================================================");
        println!("DAFTAR MIKROFON TERDETEKSI DI WINDOWS:");
        for (i, dev) in input_devices.iter().enumerate() {
            let name = dev.name().unwrap_or_else(|_| "Unknown Device".to_string());
            println!("  [{}] {}", i + 1, name);
        }

        // Select the best device: prefer physical mic (Realtek / Microphone Array) over Steam/Virtual
        let chosen_device = input_devices.iter().find(|d| {
            if let Ok(name) = d.name() {
                let lower = name.to_lowercase();
                (lower.contains("realtek")
                    || lower.contains("array")
                    || lower.contains("headset")
                    || lower.contains("microphone"))
                    && !lower.contains("steam")
                    && !lower.contains("virtual")
            } else {
                false
            }
        }).cloned().or_else(|| host.default_input_device());

        let chosen_name = chosen_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_else(|| "Default Microphone".to_string());

        println!(">> MIKROFON AKTIF DIGUNAKAN: \"{}\"", chosen_name);
        println!("==================================================");

        *active_device_name.lock() = chosen_name;

        let stream = if let Some(ref device) = chosen_device {
            Self::build_stream_for_device(
                device,
                Arc::clone(&is_recording),
                Arc::clone(&buffer),
                Arc::clone(&latest_rms),
                Arc::clone(&device_sample_rate),
                Arc::clone(&preroll_buffer),
            )
        } else {
            eprintln!("Tidak ada input device mikrofon yang ditemukan di sistem!");
            None
        };

        Self {
            is_recording,
            buffer,
            latest_rms,
            device_sample_rate,
            preroll_buffer,
            active_device_name,
            stream: Arc::new(Mutex::new(stream)),
        }
    }

    fn build_stream_for_device(
        device: &Device,
        is_rec: Arc<AtomicBool>,
        buf: Arc<Mutex<Vec<f32>>>,
        rms_target: Arc<Mutex<f32>>,
        sample_rate_target: Arc<Mutex<u32>>,
        preroll: Arc<Mutex<VecDeque<f32>>>,
    ) -> Option<Stream> {
        let config = device.default_input_config().ok()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        *sample_rate_target.lock() = sample_rate;
        let max_preroll_samples = sample_rate as usize / 6; // ~160ms

        let stream_res = match config.sample_format() {
            SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    let mut sum_sq = 0.0f32;
                    let mut count = 0;
                    let mut mono_samples = Vec::with_capacity(data.len() / channels);

                    for chunk in data.chunks(channels) {
                        let mono = chunk.iter().sum::<f32>() / (channels as f32);
                        mono_samples.push(mono);
                        sum_sq += mono * mono;
                        count += 1;
                    }

                    if count > 0 {
                        let rms = (sum_sq / count as f32).sqrt();
                        *rms_target.lock() = rms;
                    }

                    let max_recording_samples = sample_rate as usize * 60;
                    if is_rec.load(Ordering::SeqCst) {
                        let mut b = buf.lock();
                        if b.len() < max_recording_samples {
                            b.extend_from_slice(&mono_samples);
                        }
                    } else {
                        let mut pre = preroll.lock();
                        for s in mono_samples {
                            if pre.len() >= max_preroll_samples {
                                pre.pop_front();
                            }
                            pre.push_back(s);
                        }
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    let mut sum_sq = 0.0f32;
                    let mut count = 0;
                    let mut mono_samples = Vec::with_capacity(data.len() / channels);

                    for chunk in data.chunks(channels) {
                        let sum: f32 =
                            chunk.iter().map(|&s| s as f32 / i16::MAX as f32).sum();
                        let mono = sum / (channels as f32);
                        mono_samples.push(mono);
                        sum_sq += mono * mono;
                        count += 1;
                    }

                    if count > 0 {
                        let rms = (sum_sq / count as f32).sqrt();
                        *rms_target.lock() = rms;
                    }

                    let max_recording_samples = sample_rate as usize * 60;
                    if is_rec.load(Ordering::SeqCst) {
                        let mut b = buf.lock();
                        if b.len() < max_recording_samples {
                            b.extend_from_slice(&mono_samples);
                        }
                    } else {
                        let mut pre = preroll.lock();
                        for s in mono_samples {
                            if pre.len() >= max_preroll_samples {
                                pre.pop_front();
                            }
                            pre.push_back(s);
                        }
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            ),
            _ => Err(cpal::BuildStreamError::DeviceNotAvailable),
        };

        match stream_res {
            Ok(s) => {
                let _ = s.pause(); // Pause initially to turn off Windows microphone indicator
                Some(s)
            }
            Err(e) => {
                eprintln!("Gagal membangun audio stream untuk device: {}", e);
                None
            }
        }
    }

    pub fn get_available_devices() -> Vec<String> {
        let host = cpal::default_host();
        let mut list = Vec::new();
        if let Ok(devices) = host.input_devices() {
            for d in devices {
                if let Ok(name) = d.name() {
                    list.push(name);
                }
            }
        }
        list
    }

    pub fn get_active_device_name(&self) -> String {
        self.active_device_name.lock().clone()
    }

    pub fn switch_device(&self, target_name: &str) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let devices: Vec<Device> = host.input_devices()?.collect();
        let target_device = devices
            .into_iter()
            .find(|d| d.name().map(|n| n == target_name).unwrap_or(false))
            .ok_or_else(|| anyhow::anyhow!("Perangkat audio '{}' tidak ditemukan", target_name))?;

        let new_stream = Self::build_stream_for_device(
            &target_device,
            Arc::clone(&self.is_recording),
            Arc::clone(&self.buffer),
            Arc::clone(&self.latest_rms),
            Arc::clone(&self.device_sample_rate),
            Arc::clone(&self.preroll_buffer),
        );

        *self.stream.lock() = new_stream;
        *self.active_device_name.lock() = target_name.to_string();
        println!(">> Berhasil beralih ke Mikrofon: \"{}\"", target_name);
        Ok(())
    }

    pub fn start_recording(&self) -> anyhow::Result<()> {
        let mut buf = self.buffer.lock();
        buf.clear();
        let pre = self.preroll_buffer.lock();
        buf.extend(pre.iter().copied());
        self.is_recording.store(true, Ordering::SeqCst);

        // Resume audio stream instantly
        if let Some(ref s) = *self.stream.lock() {
            let _ = s.play();
        }
        Ok(())
    }

    pub fn stop_recording(&self) -> Vec<f32> {
        self.is_recording.store(false, Ordering::SeqCst);

        // Pause audio stream to release Windows mic indicator
        if let Some(ref s) = *self.stream.lock() {
            let _ = s.pause();
        }
        *self.latest_rms.lock() = 0.0;

        let raw_samples = std::mem::take(&mut *self.buffer.lock());
        let orig_rate = *self.device_sample_rate.lock();

        if raw_samples.is_empty() {
            return Vec::new();
        }

        // 1. Resample to 16,000 Hz using high-quality Rubato resampler
        let resampled = if orig_rate == TARGET_SAMPLE_RATE {
            raw_samples
        } else {
            Self::resample_rubato(&raw_samples, orig_rate, TARGET_SAMPLE_RATE)
        };

        // 2. Peak normalize
        Self::normalize_audio(resampled)
    }

    pub fn get_latest_rms(&self) -> f32 {
        *self.latest_rms.lock()
    }

    fn resample_rubato(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        if from_rate == to_rate || samples.is_empty() || from_rate == 0 || to_rate == 0 {
            return samples.to_vec();
        }

        let params = SincInterpolationParameters {
            sinc_len: 64,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: WindowFunction::BlackmanHarris2,
        };

        let chunk_size = 1024;
        let mut resampler = match SincFixedIn::<f32>::new(
            to_rate as f64 / from_rate as f64,
            2.0,
            params,
            chunk_size,
            1,
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Gagal inisialisasi Rubato resampler: {}, fallback ke linear", e);
                return Self::resample_linear_fallback(samples, from_rate, to_rate);
            }
        };

        let mut output = Vec::new();
        let mut pos = 0;

        while pos + chunk_size <= samples.len() {
            let chunk = vec![samples[pos..pos + chunk_size].to_vec()];
            if let Ok(res) = resampler.process(&chunk, None) {
                if !res.is_empty() {
                    output.extend_from_slice(&res[0]);
                }
            }
            pos += chunk_size;
        }

        // Process leftover samples with zero-padding
        if pos < samples.len() {
            let mut last_chunk = samples[pos..].to_vec();
            last_chunk.resize(chunk_size, 0.0);
            if let Ok(res) = resampler.process(&[last_chunk], None) {
                if !res.is_empty() {
                    let expected_remaining = (((samples.len() - pos) as f64)
                        * (to_rate as f64 / from_rate as f64))
                        .round() as usize;
                    let valid_len = expected_remaining.min(res[0].len());
                    output.extend_from_slice(&res[0][..valid_len]);
                }
            }
        }

        output
    }

    fn resample_linear_fallback(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        let ratio = from_rate as f64 / to_rate as f64;
        let target_len = ((samples.len() as f64) / ratio).floor() as usize;
        let mut out = Vec::with_capacity(target_len);

        for i in 0..target_len {
            let src_idx = (i as f64) * ratio;
            let idx0 = src_idx.floor() as usize;
            let idx1 = (idx0 + 1).min(samples.len() - 1);
            let frac = (src_idx - idx0 as f64) as f32;

            let s0 = samples[idx0];
            let s1 = samples[idx1];
            out.push(s0 + frac * (s1 - s0));
        }

        out
    }

    fn normalize_audio(mut samples: Vec<f32>) -> Vec<f32> {
        if samples.is_empty() {
            return samples;
        }

        let mut max_abs: f32 = 0.0;
        for &s in &samples {
            let abs = s.abs();
            if abs > max_abs {
                max_abs = abs;
            }
        }

        if max_abs > 0.02 {
            let target_peak = 0.90f32;
            let gain = (target_peak / max_abs).min(8.0);
            for s in &mut samples {
                *s *= gain;
            }
        }

        samples
    }
}
