use anyhow::{Context, Result};
use futures_util::StreamExt;
use hound::{WavSpec, WavWriter};
use parking_lot::Mutex;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

pub enum ModelSize {
    Tiny,
    Base,
    Small,
}

impl ModelSize {
    pub fn download_url(&self) -> &'static str {
        match self {
            ModelSize::Tiny => {
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
            }
            ModelSize::Base => {
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"
            }
            ModelSize::Small => {
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
            }
        }
    }

    pub fn file_name(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "ggml-tiny.bin",
            ModelSize::Base => "ggml-base.bin",
            ModelSize::Small => "ggml-small.bin",
        }
    }

    pub fn min_bytes(&self) -> u64 {
        match self {
            ModelSize::Tiny => 70_000_000,
            ModelSize::Base => 140_000_000,
            ModelSize::Small => 450_000_000,
        }
    }
}

pub struct WhisperEngine {
    cli_path: Arc<Mutex<Option<PathBuf>>>,
    model_path: Arc<Mutex<Option<PathBuf>>>,
    active_model: Arc<Mutex<String>>,
}

unsafe impl Send for WhisperEngine {}
unsafe impl Sync for WhisperEngine {}

impl Default for WhisperEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WhisperEngine {
    pub fn new() -> Self {
        Self {
            cli_path: Arc::new(Mutex::new(None)),
            model_path: Arc::new(Mutex::new(None)),
            active_model: Arc::new(Mutex::new("base".to_string())),
        }
    }

    fn get_base_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("VoiceTyping")
    }

    fn get_models_dir() -> PathBuf {
        Self::get_base_dir().join("models_ggml")
    }

    fn get_bin_dir() -> PathBuf {
        Self::get_base_dir().join("bin")
    }

    async fn download_file_atomic(url: &str, dest: &Path, min_expected_size: u64) -> Result<()> {
        if dest.exists() {
            if let Ok(meta) = dest.metadata() {
                if meta.len() >= min_expected_size {
                    return Ok(());
                }
            }
            let _ = remove_file(dest);
        }

        if let Some(parent) = dest.parent() {
            create_dir_all(parent)?;
        }

        let tmp_path = dest.with_extension("download_tmp");
        if tmp_path.exists() {
            let _ = remove_file(&tmp_path);
        }

        println!("Mengunduh dari {url} ke {dest:?} ...");
        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .send()
            .await
            .context(format!("Gagal menghubungi URL: {url}"))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP request gagal dengan status code: {}",
                response.status()
            ));
        }

        let mut file = File::create(&tmp_path)?;
        let mut stream = response.bytes_stream();
        let mut downloaded_bytes = 0u64;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Error saat menerima chunk stream")?;
            file.write_all(&chunk)?;
            downloaded_bytes += chunk.len() as u64;
        }

        file.flush()?;
        drop(file);

        if downloaded_bytes < min_expected_size {
            let _ = remove_file(&tmp_path);
            return Err(anyhow::anyhow!(
                "Ukuran unduhan terlalu kecil ({downloaded_bytes} bytes), minimal {min_expected_size} bytes"
            ));
        }

        rename(&tmp_path, dest)?;
        println!("Selesai mengunduh {dest:?} ({downloaded_bytes} bytes)");
        Ok(())
    }

    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        let size = match model_name {
            "tiny" => ModelSize::Tiny,
            "small" => ModelSize::Small,
            _ => ModelSize::Base,
        };

        // 1. Locate whisper-cli.exe in bundled resources, current exe dir, workspace or appdata
        let current_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let exe_candidates = [
            current_dir.as_ref().map(|d| d.join("resources").join("bin").join("Release").join("whisper-cli.exe")).unwrap_or_default(),
            current_dir.as_ref().map(|d| d.join("resources").join("bin").join("whisper-cli.exe")).unwrap_or_default(),
            current_dir.as_ref().map(|d| d.join("bin").join("Release").join("whisper-cli.exe")).unwrap_or_default(),
            current_dir.as_ref().map(|d| d.join("whisper-cli.exe")).unwrap_or_default(),
            PathBuf::from("src-tauri/resources/bin/Release/whisper-cli.exe"),
            PathBuf::from("src-tauri/resources/bin/whisper-cli.exe"),
            PathBuf::from("src-tauri/bin/Release/whisper-cli.exe"),
            PathBuf::from("bin/Release/whisper-cli.exe"),
            Self::get_bin_dir().join("Release").join("whisper-cli.exe"),
            Self::get_bin_dir().join("whisper-cli.exe"),
        ];

        let mut found_cli: Option<PathBuf> = None;
        for cand in &exe_candidates {
            if cand.as_os_str().is_empty() {
                continue;
            }
            if cand.exists() {
                if let Ok(abs) = cand.canonicalize() {
                    found_cli = Some(abs);
                    break;
                }
            }
        }

        let cli = match found_cli {
            Some(p) => p,
            None => {
                let zip_dest = Self::get_bin_dir().join("whisper-bin.zip");
                let zip_url = "https://github.com/ggerganov/whisper.cpp/releases/latest/download/whisper-bin-x64.zip";
                Self::download_file_atomic(zip_url, &zip_dest, 1_000_000).await?;

                let bin_dir = Self::get_bin_dir();
                let file = File::open(&zip_dest)?;
                let mut archive = zip::ZipArchive::new(file)?;
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let outpath = match file.enclosed_name() {
                        Some(path) => bin_dir.join(path),
                        None => continue,
                    };
                    if file.is_dir() {
                        create_dir_all(&outpath)?;
                    } else {
                        if let Some(p) = outpath.parent() {
                            if !p.exists() {
                                create_dir_all(p)?;
                            }
                        }
                        let mut outfile = File::create(&outpath)?;
                        std::io::copy(&mut file, &mut outfile)?;
                    }
                }
                bin_dir.join("Release").join("whisper-cli.exe")
            }
        };

        // 2. Locate or copy model file from bundled resources
        let models_dir = Self::get_models_dir();
        let model_file = models_dir.join(size.file_name());

        if !model_file.exists() {
            let bundled_candidates = [
                current_dir.as_ref().map(|d| d.join("resources").join("models_ggml").join(size.file_name())).unwrap_or_default(),
                PathBuf::from("src-tauri/resources/models_ggml").join(size.file_name()),
            ];
            for bundled in &bundled_candidates {
                if !bundled.as_os_str().is_empty() && bundled.exists() {
                    if let Some(p) = model_file.parent() {
                        let _ = create_dir_all(p);
                    }
                    println!("Menemukan model offline di {bundled:?}, menyalin ke {model_file:?} ...");
                    let _ = std::fs::copy(bundled, &model_file);
                    break;
                }
            }
        }

        if !model_file.exists() {
            Self::download_file_atomic(size.download_url(), &model_file, size.min_bytes()).await?;
        }

        *self.cli_path.lock() = Some(cli);
        *self.model_path.lock() = Some(model_file);
        *self.active_model.lock() = model_name.to_string();

        println!("Whisper Engine (whisper.cpp) siap digunakan!");
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.model_path.lock().is_some() && self.cli_path.lock().is_some()
    }

    pub fn transcribe(&self, raw_samples: &[f32], language: Option<&str>) -> Result<String> {
        if raw_samples.is_empty() {
            return Ok(String::new());
        }

        let cli_guard = self.cli_path.lock();
        let cli = cli_guard
            .as_ref()
            .context("whisper-cli belum diinisialisasi.")?;

        let model_guard = self.model_path.lock();
        let model = model_guard
            .as_ref()
            .context("Model ggml belum dimuat.")?;

        // 1. Write temporary 16kHz mono WAV file
        let temp_dir = std::env::temp_dir().join("VoiceTyping");
        create_dir_all(&temp_dir)?;
        let temp_wav = temp_dir.join(format!("rec_{}.wav", std::process::id()));

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(&temp_wav, spec)?;
        for &s in raw_samples {
            let clamped = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(clamped)?;
        }
        writer.finalize()?;

        struct TempFileGuard<'a>(&'a Path);
        impl<'a> Drop for TempFileGuard<'a> {
            fn drop(&mut self) {
                let _ = remove_file(self.0);
            }
        }
        let _guard = TempFileGuard(&temp_wav);

        // 2. Call whisper-cli.exe
        let lang = match language {
            Some("id") => "id",
            Some("en") => "en",
            _ => "auto",
        };

        let threads = std::thread::available_parallelism()
            .map(|n| n.get().clamp(4, 8))
            .unwrap_or(4);

        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        #[cfg(windows)]
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new(cli);
        cmd.arg("-m")
            .arg(model)
            .arg("-f")
            .arg(&temp_wav)
            .arg("-l")
            .arg(lang)
            .arg("-t")
            .arg(threads.to_string())
            .arg("--prompt")
            .arg("Halo, ini adalah aplikasi voice typing bahasa Indonesia.")
            .arg("-nt") // no timestamps
            .arg("-np") // no prints (only output transcribed text)
            .arg("--flash-attn"); // hardware SIMD acceleration

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().context("Gagal menjalankan whisper-cli.exe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("whisper-cli error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut cleaned = stdout.trim().to_string();
        for tag in ["[BLANK_AUDIO]", "[MUSIC]", "(music)", "[laughter]", "[silence]", "[applause]"] {
            cleaned = cleaned.replace(tag, "");
        }
        let final_text = cleaned.trim().to_string();

        Ok(final_text)
    }
}
