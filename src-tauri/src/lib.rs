mod audio;
mod models;
mod paste;
mod settings;
mod transcribe;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_updater::UpdaterExt;

/// Всё, что программа держит в памяти.
struct AppState {
    dir: PathBuf,
    settings: Mutex<settings::Settings>,
    recorder: Mutex<Option<audio::Recorder>>,
    engine: Mutex<Option<transcribe::Engine>>,
    history: Mutex<Vec<String>>,
    busy: Mutex<bool>,
    live: Mutex<Live>,
}

/// Живой режим: сколько слов уже вписано в поле в текущей записи.
#[derive(Default)]
struct Live {
    session: u64,
    active: bool,
    typed: usize,
}

impl AppState {
    fn models_dir(&self) -> PathBuf {
        self.dir.join("models")
    }
}

#[derive(Serialize, Clone)]
struct Status {
    /// "listening" | "thinking" | "done" | "error" | "idle"
    state: String,
    text: String,
}

fn status(app: &AppHandle, state: &str, text: &str) {
    let _ = app.emit("status", Status { state: state.into(), text: text.into() });
    if let Some(w) = app.get_webview_window("overlay") {
        if state == "idle" {
            let _ = w.hide();
        } else {
            place_overlay(app);
            let _ = w.show();
        }
    }
}

/// Ставим индикатор внизу по центру экрана, где сейчас курсор/главный монитор.
fn place_overlay(app: &AppHandle) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let monitor = w.current_monitor().ok().flatten().or_else(|| w.primary_monitor().ok().flatten());
    if let (Some(m), Ok(size)) = (monitor, w.outer_size()) {
        let ms = m.size();
        let mp = m.position();
        let x = mp.x + (ms.width as i32 - size.width as i32) / 2;
        let y = mp.y + ms.height as i32 - size.height as i32 - (ms.height as f64 * 0.08) as i32;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    let _ = w.set_ignore_cursor_events(true);
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

// ---------- запись и распознавание ----------

fn start_recording(app: &AppHandle) {
    let st = app.state::<AppState>();
    if *st.busy.lock().unwrap() {
        return;
    }
    let model = st.settings.lock().unwrap().model.clone();
    if !models::is_downloaded(&st.models_dir(), &model) {
        status(app, "error", "Сначала скачай модель");
        show_main(app);
        hide_later(app, 2500);
        return;
    }
    let mut rec = st.recorder.lock().unwrap();
    if rec.is_some() {
        return;
    }
    match audio::Recorder::start() {
        Ok(r) => {
            *rec = Some(r);
            drop(rec);
            status(app, "listening", "Слушаю…");
            if st.settings.lock().unwrap().live {
                let sid = {
                    let mut l = st.live.lock().unwrap();
                    l.session += 1;
                    l.active = true;
                    l.typed = 0;
                    l.session
                };
                let app = app.clone();
                std::thread::spawn(move || live_loop(app, sid));
            }
        }
        Err(e) => {
            status(app, "error", &e.to_string());
            hide_later(app, 3000);
        }
    }
}

fn stop_recording(app: &AppHandle) {
    let st = app.state::<AppState>();
    let Some(rec) = st.recorder.lock().unwrap().take() else { return };
    *st.busy.lock().unwrap() = true;
    status(app, "thinking", "Распознаю…");

    let app = app.clone();
    std::thread::spawn(move || {
        let st = app.state::<AppState>();
        let samples = rec.stop();
        let result = if !audio::has_voice(&samples) {
            Ok(String::new())
        } else {
            transcribe_samples(&app, &samples)
        };
        // Живой режим выключаем и узнаём, сколько слов он уже вписал
        let typed = {
            let mut l = st.live.lock().unwrap();
            l.active = false;
            std::mem::take(&mut l.typed)
        };
        match result {
            Ok(text) if !text.is_empty() => {
                let insert = if typed > 0 {
                    let rest: Vec<&str> = text.split_whitespace().skip(typed).collect();
                    if rest.is_empty() { Ok(()) } else { paste::type_text(&format!(" {}", rest.join(" "))) }
                } else {
                    paste::paste_text(&text)
                };
                if let Err(e) = insert {
                    status(&app, "error", &format!("Не получилось вставить: {e}"));
                    let _ = paste::copy_text(&text);
                } else {
                    status(&app, "done", "Готово");
                }
                let mut h = st.history.lock().unwrap();
                h.insert(0, text);
                h.truncate(20);
                let _ = app.emit("history", h.clone());
            }
            Ok(_) => status(&app, "error", "Ничего не услышал"),
            Err(e) => status(&app, "error", &e.to_string()),
        }
        *st.busy.lock().unwrap() = false;
        hide_later(&app, 900);
    });
}

/// Живой режим: каждые ~0,8 с распознаём всё сказанное и показываем/вписываем
/// только «устоявшиеся» слова — те, что не поменялись между двумя проходами.
fn live_loop(app: AppHandle, sid: u64) {
    let mut prev: Vec<String> = Vec::new();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(800));
        let st = app.state::<AppState>();
        {
            let l = st.live.lock().unwrap();
            if l.session != sid || !l.active { break; }
        }
        let snap = match st.recorder.lock().unwrap().as_ref() {
            Some(r) => r.snapshot(),
            None => break,
        };
        if snap.len() < audio::TARGET_RATE as usize || !audio::has_voice(&snap) {
            continue; // меньше секунды или тишина
        }
        let Ok(text) = transcribe_samples(&app, &snap) else { continue };
        let words: Vec<String> = text.split_whitespace().map(String::from).collect();

        // Субтитры в индикаторе — последние слова
        let tail: String = {
            let t = words.join(" ");
            let n = t.chars().count();
            if n > 70 { format!("…{}", t.chars().skip(n - 70).collect::<String>()) } else { t }
        };
        let mut l = st.live.lock().unwrap();
        if l.session != sid || !l.active { break; }
        if !tail.is_empty() {
            let _ = app.emit("status", Status { state: "listening".into(), text: tail });
        }

        // Вписываем в поле только в режиме «нажал-начал / нажал-закончил»:
        // в режиме «держу клавишу» зажатые ⌘/Ctrl смешались бы с буквами.
        let toggle = st.settings.lock().unwrap().mode == "toggle";
        if toggle {
            let same = words.iter().zip(prev.iter()).take_while(|(a, b)| norm(a) == norm(b)).count();
            let stable = same.min(words.len().saturating_sub(1)); // последнее слово ещё может поменяться
            if stable > l.typed {
                let chunk = words[l.typed..stable].join(" ");
                let piece = if l.typed > 0 { format!(" {chunk}") } else { chunk };
                if paste::type_text(&piece).is_ok() {
                    l.typed = stable;
                }
            }
        }
        drop(l);
        prev = words;
    }
}

/// Слово без регистра и знаков препинания — чтобы «Привет,» и «привет» считались одним словом.
fn norm(w: &str) -> String {
    w.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn transcribe_samples(app: &AppHandle, samples: &[f32]) -> anyhow::Result<String> {
    let st = app.state::<AppState>();
    let (model, language) = {
        let s = st.settings.lock().unwrap();
        (s.model.clone(), s.language.clone())
    };
    let mut engine = st.engine.lock().unwrap();
    // Загружаем модель при первом использовании или после смены
    if engine.as_ref().map(|e| e.model_id != model).unwrap_or(true) {
        let path = models::file_path(&st.models_dir(), &model)
            .ok_or_else(|| anyhow::anyhow!("Нет модели"))?;
        *engine = Some(transcribe::Engine::load(&model, &path)?);
    }
    engine.as_ref().unwrap().transcribe(samples, &language)
}

fn hide_later(app: &AppHandle, ms: u64) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        let st = app.state::<AppState>();
        if st.recorder.lock().unwrap().is_none() && !*st.busy.lock().unwrap() {
            status(&app, "idle", "");
        }
    });
}

// ---------- горячая клавиша ----------

fn register_hotkey(app: &AppHandle) -> Result<(), String> {
    let st = app.state::<AppState>();
    let hotkey = st.settings.lock().unwrap().hotkey.clone();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(hotkey.as_str())
        .map_err(|e| format!("Клавиша «{hotkey}» занята или не подходит: {e}"))
}

fn on_hotkey(app: &AppHandle, state: ShortcutState) {
    let mode = app.state::<AppState>().settings.lock().unwrap().mode.clone();
    let recording = app.state::<AppState>().recorder.lock().unwrap().is_some();
    match (mode.as_str(), state) {
        ("toggle", ShortcutState::Pressed) => {
            if recording { stop_recording(app) } else { start_recording(app) }
        }
        ("toggle", ShortcutState::Released) => {}
        (_, ShortcutState::Pressed) => start_recording(app),
        (_, ShortcutState::Released) => stop_recording(app),
    }
}

// ---------- команды для окна ----------

// ---------- автообновление ----------

/// Проверить обновление. install=true — сразу скачать, поставить и перезапустить.
async fn check_update(app: AppHandle, install: bool) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| format!("Не удалось проверить: {e}"))? else {
        return Ok(None);
    };
    let version = update.version.clone();
    if !install {
        return Ok(Some(version));
    }
    let _ = app.emit("update", serde_json::json!({ "state": "downloading", "version": version }));
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Не удалось скачать обновление: {e}"))?;
    // Не перезапускаемся посреди диктовки — ждём, пока человек договорит
    loop {
        let st = app.state::<AppState>();
        let idle = st.recorder.lock().unwrap().is_none() && !*st.busy.lock().unwrap();
        if idle { break; }
        tokio_sleep(2000).await;
    }
    update.install(bytes).map_err(|e| format!("Не удалось установить: {e}"))?;
    app.restart();
}

async fn tokio_sleep(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

/// Фоновая проверка: при запуске и потом раз в 6 часов.
fn start_auto_update(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio_sleep(15_000).await; // даём программе спокойно стартовать
        loop {
            let enabled = app.state::<AppState>().settings.lock().unwrap().auto_update;
            if enabled {
                let _ = check_update(app.clone(), true).await;
            }
            tokio_sleep(6 * 60 * 60 * 1000).await;
        }
    });
}

#[tauri::command]
async fn update_now(app: AppHandle) -> Result<String, String> {
    match check_update(app, true).await? {
        Some(v) => Ok(v),
        None => Ok(String::new()),
    }
}

#[derive(Serialize)]
struct UiState {
    version: String,
    settings: settings::Settings,
    models: Vec<models::ModelView>,
    history: Vec<String>,
    platform: String,
}

#[tauri::command]
fn get_state(app: AppHandle, st: State<AppState>) -> UiState {
    UiState {
        version: app.package_info().version.to_string(),
        settings: st.settings.lock().unwrap().clone(),
        models: models::list(&st.models_dir()),
        history: st.history.lock().unwrap().clone(),
        platform: std::env::consts::OS.into(),
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, st: State<AppState>, new: settings::Settings) -> Result<(), String> {
    let old = st.settings.lock().unwrap().clone();
    *st.settings.lock().unwrap() = new.clone();
    settings::save(&st.dir, &new);
    if old.autostart != new.autostart {
        let al = app.autolaunch();
        let _ = if new.autostart { al.enable() } else { al.disable() };
    }
    if old.hotkey != new.hotkey {
        if let Err(e) = register_hotkey(&app) {
            // возвращаем старую клавишу
            st.settings.lock().unwrap().hotkey = old.hotkey.clone();
            settings::save(&st.dir, &st.settings.lock().unwrap());
            let _ = register_hotkey(&app);
            return Err(e);
        }
    }
    Ok(())
}

#[tauri::command]
async fn download_model(app: AppHandle, id: String) -> Result<(), String> {
    let dir = app.state::<AppState>().models_dir();
    let app2 = app.clone();
    let id2 = id.clone();
    models::download(&dir, &id, move |done, total| {
        let _ = app2.emit("download-progress", serde_json::json!({ "id": id2, "done": done, "total": total }));
    })
    .await
    .map_err(|e| format!("Не получилось скачать: {e}"))
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    paste::copy_text(&text).map_err(|e| e.to_string())
}

// ---------- запуск ----------

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| on_hotkey(app, event.state()))
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let dir = app.path().app_data_dir()?;
            let s = settings::load(&dir);
            let onboarded = s.onboarded;
            app.manage(AppState {
                dir,
                settings: Mutex::new(s),
                recorder: Mutex::new(None),
                engine: Mutex::new(None),
                history: Mutex::new(Vec::new()),
                busy: Mutex::new(false),
                live: Mutex::new(Live::default()),
            });

            if let Err(e) = register_hotkey(app.handle()) {
                eprintln!("{e}");
            }
            start_auto_update(app.handle().clone());

            // Иконка в трее / строке меню
            let open = MenuItem::with_id(app, "open", "Настройки и история", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Shepot — голос в текст")
                .menu(&menu)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Закрытие окна настроек прячет его, а не выключает программу
            if let Some(w) = app.get_webview_window("main") {
                let w2 = w.clone();
                w.on_window_event(move |e| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        let _ = w2.hide();
                    }
                });
                if !onboarded {
                    let _ = w.show();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_state, save_settings, download_model, copy_text, update_now])
        .build(tauri::generate_context!())
        .expect("не удалось запустить Shepot")
        .run(|app, event| {
            // Повторный запуск / клик по иконке в Dock — открыть окно
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main(app);
            }
            let _ = (app, event);
        });
}
