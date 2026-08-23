use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};
use tokio::time::sleep;

pub mod asr;
pub mod audio;
pub mod cleaner;
pub mod hotkey;
pub mod injector;

use asr::WhisperEngine;
use audio::AudioRecorder;
use cleaner::TextCleaner;
use hotkey::HotkeyListener;
use injector::TextInjector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub text: String,
    pub timestamp: u64,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub model_name: String,
    pub is_model_loaded: bool,
    pub language_mode: String,
    pub is_recording: bool,
    pub active_mic: String,
}

pub struct AppState {
    pub recorder: Arc<AudioRecorder>,
    pub whisper: Arc<WhisperEngine>,
    pub cleaner: Arc<TextCleaner>,
    pub language_mode: Arc<Mutex<String>>,
    pub model_name: Arc<Mutex<String>>,
    pub is_recording: Arc<AtomicBool>,
    pub history: Arc<Mutex<Vec<HistoryItem>>>,
    pub _hotkey: Mutex<Option<HotkeyListener>>,
}

#[tauri::command]
fn get_app_status(state: State<'_, AppState>) -> AppStatus {
    AppStatus {
        model_name: state.model_name.lock().clone(),
        is_model_loaded: state.whisper.is_loaded(),
        language_mode: state.language_mode.lock().clone(),
        is_recording: state.is_recording.load(Ordering::SeqCst),
        active_mic: state.recorder.get_active_device_name(),
    }
}

#[tauri::command]
fn get_audio_devices() -> Vec<String> {
    AudioRecorder::get_available_devices()
}

#[tauri::command]
fn set_audio_device(name: String, state: State<'_, AppState>) -> Result<(), String> {
    state.recorder.switch_device(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_language_mode(mode: String, state: State<'_, AppState>) -> Result<(), String> {
    *state.language_mode.lock() = mode;
    Ok(())
}

#[tauri::command]
async fn switch_model(name: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .whisper
        .load_model(&name)
        .await
        .map_err(|e| e.to_string())?;
    *state.model_name.lock() = name;
    Ok(())
}

#[tauri::command]
fn get_history(state: State<'_, AppState>) -> Vec<HistoryItem> {
    state.history.lock().clone()
}

#[tauri::command]
fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history.lock().clear();
    Ok(())
}

#[tauri::command]
fn get_custom_dictionary(state: State<'_, AppState>) -> std::collections::HashMap<String, String> {
    state.cleaner.get_custom_dict()
}

#[tauri::command]
fn save_custom_dictionary(dict: std::collections::HashMap<String, String>, state: State<'_, AppState>) -> Result<(), String> {
    state.cleaner.save_custom_dict(dict)
}

#[tauri::command]
fn toggle_settings_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let recorder = Arc::new(AudioRecorder::new());
            let whisper = Arc::new(WhisperEngine::new());
            let cleaner = Arc::new(TextCleaner::new());
            let language_mode = Arc::new(Mutex::new("auto".to_string()));
            let model_name = Arc::new(Mutex::new("small".to_string()));
            let is_recording = Arc::new(AtomicBool::new(false));
            let history = Arc::new(Mutex::new(Vec::<HistoryItem>::new()));

            let app_handle = app.handle().clone();

            // Setup Native Windows System Tray Submenus (TranslucentTB style)
            let about_i = MenuItem::with_id(app, "about", "🕷️ TypeLess v0.1 • Local AI (Small)", true, None::<&str>)?;

            // Microphone Input Submenu
            let available_mics = AudioRecorder::get_available_devices();
            let active_mic_name = recorder.get_active_device_name();

            let mut mic_check_items = Vec::new();
            for (idx, mic_name) in available_mics.iter().enumerate() {
                let is_checked = mic_name == &active_mic_name;
                let menu_id = format!("mic_{}", idx);
                let item = CheckMenuItem::with_id(app, &menu_id, mic_name, true, is_checked, None::<&str>)?;
                mic_check_items.push(item);
            }

            let mic_items_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = mic_check_items
                .iter()
                .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
                .collect();

            let mic_submenu = Submenu::with_items(app, "🎙️ Microphone Input", true, &mic_items_refs)?;

            // Language Submenu
            let lang_auto = CheckMenuItem::with_id(app, "lang_auto", "Auto Detect (ID / EN)", true, true, None::<&str>)?;
            let lang_id = CheckMenuItem::with_id(app, "lang_id", "Bahasa Indonesia (ID)", true, false, None::<&str>)?;
            let lang_en = CheckMenuItem::with_id(app, "lang_en", "English (EN)", true, false, None::<&str>)?;
            let lang_submenu = Submenu::with_items(app, "🌐 Language Mode", true, &[&lang_auto, &lang_id, &lang_en])?;

            // Hotkey Submenu
            let hk_rctrl = CheckMenuItem::with_id(app, "hk_rctrl", "Right Ctrl (Push-to-Talk)", true, true, None::<&str>)?;
            let hk_ralt = CheckMenuItem::with_id(app, "hk_ralt", "Right Alt (Push-to-Talk)", true, false, None::<&str>)?;
            let hk_f8 = CheckMenuItem::with_id(app, "hk_f8", "F8 Key (Push-to-Talk)", true, false, None::<&str>)?;
            let hotkey_submenu = Submenu::with_items(app, "⌨️ Hotkey / Shortcut", true, &[&hk_rctrl, &hk_ralt, &hk_f8])?;

            let quit_i = MenuItem::with_id(app, "quit", "⏻ Exit TypeLess", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&about_i, &mic_submenu, &lang_submenu, &hotkey_submenu, &quit_i],
            )?;

            let available_mics_for_event = available_mics.clone();

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("TypeLess (Hold Right Ctrl to Speak)")
                .on_menu_event(move |app, event| {
                    let id = event.id.as_ref();

                    if let Some(idx_str) = id.strip_prefix("mic_") {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if let Some(target_mic) = available_mics_for_event.get(idx) {
                                if let Some(state) = app.try_state::<AppState>() {
                                    if let Err(e) = state.recorder.switch_device(target_mic) {
                                        eprintln!("Gagal mengganti mikrofon: {}", e);
                                    } else {
                                        println!("Mikrofon aktif berhasil dialihkan ke: \"{}\"", target_mic);
                                    }
                                }
                            }
                        }
                    } else {
                        match id {
                            "lang_auto" => {
                                if let Some(state) = app.try_state::<AppState>() {
                                    *state.language_mode.lock() = "auto".to_string();
                                    println!("Language mode changed to: Auto Detect");
                                }
                            }
                            "lang_id" => {
                                if let Some(state) = app.try_state::<AppState>() {
                                    *state.language_mode.lock() = "id".to_string();
                                    println!("Language mode changed to: Bahasa Indonesia");
                                }
                            }
                            "lang_en" => {
                                if let Some(state) = app.try_state::<AppState>() {
                                    *state.language_mode.lock() = "en".to_string();
                                    println!("Language mode changed to: English");
                                }
                            }
                            "quit" => {
                                app.exit(0);
                            }
                            _ => {}
                        }
                    }
                })
                .build(app)?;

            // Position floating Spidey overlay at bottom-right of primary monitor (above Windows taskbar)
            if let Some(pill_window) = app.get_webview_window("pill") {
                if let Ok(Some(monitor)) = pill_window.primary_monitor() {
                    let screen_size = monitor.size();
                    let overlay_width = 260;
                    let overlay_height = 110;
                    let x = (screen_size.width as i32) - overlay_width - 24;
                    let y = (screen_size.height as i32) - overlay_height - 60; // Position above taskbar
                    let _ = pill_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
                }
            }

            // Spawn Background model loader (loads whisper small on startup)
            let whisper_loader = Arc::clone(&whisper);
            let app_handle_for_loader = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                println!("Memuat model Whisper Small (Akurasi Tinggi) ke memori...");
                match whisper_loader.load_model("small").await {
                    Ok(_) => {
                        println!("Model Whisper Small berhasil dimuat & siap digunakan!");
                        let _ = app_handle_for_loader.emit("model-ready", true);
                    }
                    Err(e) => {
                        eprintln!("Gagal memuat model Whisper Small: {}", e);
                        let _ = app_handle_for_loader.emit("model-error", e.to_string());
                    }
                }
            });

            // Setup Hotkey Pipeline Callback (Right Ctrl Push-to-Talk)
            let rec_for_hook = Arc::clone(&recorder);
            let is_rec_for_hook = Arc::clone(&is_recording);
            let app_handle_for_hook = app_handle.clone();
            let whisper_for_hook = Arc::clone(&whisper);
            let cleaner_for_hook = Arc::clone(&cleaner);
            let lang_for_hook = Arc::clone(&language_mode);
            let hist_for_hook = Arc::clone(&history);

            let hotkey_callback = Arc::new(move |pressed: bool| {
                let app = app_handle_for_hook.clone();
                let rec = rec_for_hook.clone();
                let is_rec = is_rec_for_hook.clone();
                let whisper = whisper_for_hook.clone();
                let cleaner = cleaner_for_hook.clone();
                let lang_mode = lang_for_hook.clone();
                let hist = hist_for_hook.clone();

                if pressed {
                    println!("[TypeLess] Hotkey Pressed -> Mulai merekam & menampilkan Spidey Pet...");
                    // Push-to-Talk: Key Down (Start recording)
                    is_rec.store(true, Ordering::SeqCst);
                    if let Err(e) = rec.start_recording() {
                        eprintln!("Gagal memulai rekaman audio: {}", e);
                        return;
                    }

                    // Show floating Spidey Pet Overlay
                    if let Some(pill) = app.get_webview_window("pill") {
                        if let Ok(Some(monitor)) = pill.primary_monitor() {
                            let screen_size = monitor.size();
                            let scale_factor = monitor.scale_factor();
                            let overlay_width = (320.0 * scale_factor) as i32;
                            let overlay_height = (160.0 * scale_factor) as i32;
                            let x = (screen_size.width as i32) - overlay_width - (16.0 * scale_factor) as i32;
                            let y = (screen_size.height as i32) - overlay_height - (56.0 * scale_factor) as i32;
                            let _ = pill.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
                        }
                        let _ = pill.show();
                        let _ = pill.set_always_on_top(true);
                    }
                    let _ = app.emit("pill-state", serde_json::json!({
                        "status": "listening",
                        "text": "Mendengarkan..."
                    }));

                    // Spawn RMS audio stream loop
                    let rec_rms = rec.clone();
                    let app_rms = app.clone();
                    let is_rec_rms = is_rec.clone();
                    tauri::async_runtime::spawn(async move {
                        while is_rec_rms.load(Ordering::SeqCst) {
                            let rms = rec_rms.get_latest_rms();
                            let _ = app_rms.emit("audio-rms", rms);
                            sleep(Duration::from_millis(40)).await; // 25 fps
                        }
                    });
                } else {
                    println!("[TypeLess] Hotkey Released -> Memproses suara & transkripsi...");
                    // Push-to-Talk: Key Up (Stop recording & transcribe)
                    is_rec.store(false, Ordering::SeqCst);
                    let samples = rec.stop_recording();

                    let _ = app.emit("pill-state", serde_json::json!({
                        "status": "transcribing",
                        "text": "Memproses suara..."
                    }));

                    tauri::async_runtime::spawn_blocking(move || {
                        if samples.len() > 1600 { // at least 0.1 second of audio
                            let duration_sec = samples.len() as f32 / 16000.0;
                            println!("Merekam {} sampel audio ({:.2} detik), mulai transkripsi...", samples.len(), duration_sec);
                            let current_lang = lang_mode.lock().clone();
                            let lang_arg = match current_lang.as_str() {
                                "id" => Some("id"),
                                "en" => Some("en"),
                                _ => None, // auto detect
                            };

                            let start = std::time::Instant::now();
                            match whisper.transcribe(&samples, lang_arg) {
                                Ok(raw_text) => {
                                    let cleaned = cleaner.clean(&raw_text);
                                    let elapsed = start.elapsed();
                                    if !cleaned.is_empty() {
                                        println!("Transkripsi Selesai ({:.2?}): {}", elapsed, cleaned);
                                        let _ = app.emit("pill-state", serde_json::json!({
                                             "status": "done",
                                            "text": cleaned.clone()
                                        }));

                                        // Inject text to active application
                                        if let Err(e) = TextInjector::paste_text(&cleaned) {
                                            eprintln!("Gagal mengetikkan teks: {}", e);
                                        }

                                        // Store in history
                                        let now = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();

                                        hist.lock().insert(
                                            0,
                                            HistoryItem {
                                                id: format!("{}-{}", now, rand_id()),
                                                text: cleaned,
                                                timestamp: now,
                                                language: current_lang,
                                            },
                                        );
                                    } else {
                                        println!("Transkripsi selesai ({:.2?}), tidak ada kata yang terdeteksi.", elapsed);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error transkripsi: {}", e);
                                    let _ = app.emit("pill-state", serde_json::json!({
                                        "status": "error",
                                        "text": format!("Error: {}", e)
                                    }));
                                }
                            }
                        } else {
                            println!("Audio terlalu singkat ({} sampel), lewati transkripsi.", samples.len());
                        }

                        // Hide pill after short delay to let user see done/wink animation
                        std::thread::sleep(Duration::from_millis(1200));
                        if let Some(pill) = app.get_webview_window("pill") {
                            let _ = pill.hide();
                        }
                    });
                }
            });

            let listener = HotkeyListener::start(hotkey_callback);

            let state = AppState {
                recorder,
                whisper,
                cleaner,
                language_mode,
                model_name,
                is_recording,
                history,
                _hotkey: Mutex::new(Some(listener)),
            };

            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_audio_devices,
            set_audio_device,
            set_language_mode,
            switch_model,
            get_history,
            clear_history,
            get_custom_dictionary,
            save_custom_dictionary,
            toggle_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn rand_id() -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    now % 10000
}
