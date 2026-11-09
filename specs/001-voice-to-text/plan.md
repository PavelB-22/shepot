# Plan
**Стек:** Tauri 2 (Rust), интерфейс — обычный HTML/CSS/JS без сборщиков (`window.__TAURI__`).

**Rust-модули (src-tauri/src):**
- `audio.rs` — запись микрофона (cpal), моно, ресемплинг в 16 кГц.
- `transcribe.rs` — whisper-rs (whisper.cpp), модель держим в памяти.
- `models.rs` — список моделей, скачивание с прогрессом (reqwest, потоково).
- `paste.rs` — буфер (arboard) + Ctrl/Cmd+V (enigo), восстановление буфера.
- `settings.rs` — настройки в JSON в папке программы.
- `lib.rs` — трей, окна, горячая клавиша (tauri-plugin-global-shortcut), команды для интерфейса.

**Окна:** `main` (настройки/приветствие/история), `overlay` (индикатор, прозрачный, поверх всех, не перехватывает мышь).

**Модели (whisper.cpp, Hugging Face ggerganov/whisper.cpp):**
- Быстро — `ggml-base-q5_1.bin` (~60 МБ)
- Хорошо — `ggml-small-q5_1.bin` (~190 МБ) — по умолчанию
- Отлично — `ggml-large-v3-turbo-q5_0.bin` (~550 МБ)

**Сборка:** GitHub Actions + tauri-action → Releases: `.dmg` (aarch64, x86_64), `.exe`/`.msi` (Windows). Проверочная сборка — Linux в облаке Claude.

**Проверка по constitution:** всё локально ✓, одна кодовая база ✓, установщик маленький (модель отдельно) ✓.
