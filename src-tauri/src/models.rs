//! Модели распознавания речи (whisper.cpp) и их скачивание.
use futures_util::StreamExt;
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct ModelInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub file: &'static str,
    pub size_mb: u32,
    pub note: &'static str,
}

pub const MODELS: &[ModelInfo] = &[
    ModelInfo { id: "base", title: "Быстро", file: "ggml-base-q5_1.bin", size_mb: 60,
        note: "Мгновенно, но может ошибаться в сложных словах" },
    ModelInfo { id: "small", title: "Хорошо", file: "ggml-small-q5_1.bin", size_mb: 190,
        note: "Золотая середина — рекомендуем" },
    ModelInfo { id: "turbo", title: "Отлично", file: "ggml-large-v3-turbo-q5_0.bin", size_mb: 550,
        note: "Максимальная точность, нужен мощный компьютер" },
];

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";

#[derive(Serialize, Clone)]
pub struct ModelView {
    pub id: String,
    pub title: String,
    pub size_mb: u32,
    pub note: String,
    pub downloaded: bool,
}

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn file_path(dir: &Path, id: &str) -> Option<PathBuf> {
    find(id).map(|m| dir.join(m.file))
}

pub fn is_downloaded(dir: &Path, id: &str) -> bool {
    file_path(dir, id).map(|p| p.exists()).unwrap_or(false)
}

pub fn list(dir: &Path) -> Vec<ModelView> {
    MODELS
        .iter()
        .map(|m| ModelView {
            id: m.id.into(),
            title: m.title.into(),
            size_mb: m.size_mb,
            note: m.note.into(),
            downloaded: is_downloaded(dir, m.id),
        })
        .collect()
}

/// Скачивает модель, вызывая `progress(скачано, всего)` по ходу.
pub async fn download(dir: &Path, id: &str, progress: impl Fn(u64, u64)) -> anyhow::Result<()> {
    let m = find(id).ok_or_else(|| anyhow::anyhow!("нет такой модели"))?;
    std::fs::create_dir_all(dir)?;
    let final_path = dir.join(m.file);
    let tmp_path = dir.join(format!("{}.part", m.file));

    let resp = reqwest::get(format!("{BASE_URL}{}", m.file)).await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(m.size_mb as u64 * 1024 * 1024);
    let mut file = std::fs::File::create(&tmp_path)?;
    let mut done: u64 = 0;
    let mut last_report: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        done += chunk.len() as u64;
        if done - last_report > 512 * 1024 || done == total {
            progress(done, total);
            last_report = done;
        }
    }
    file.flush()?;
    drop(file);
    std::fs::rename(&tmp_path, &final_path)?;
    progress(total, total);
    Ok(())
}
