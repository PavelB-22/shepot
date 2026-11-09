//! Распознавание речи через whisper.cpp.
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Engine {
    pub model_id: String,
    ctx: WhisperContext,
}

impl Engine {
    pub fn load(model_id: &str, path: &Path) -> anyhow::Result<Self> {
        let ctx = WhisperContext::new_with_params(
            path.to_str().ok_or_else(|| anyhow::anyhow!("плохой путь к модели"))?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow::anyhow!("Не удалось загрузить модель: {e:?}"))?;
        Ok(Self { model_id: model_id.into(), ctx })
    }

    /// language: "auto", "ru", "en"
    pub fn transcribe(&self, audio: &[f32], language: &str) -> anyhow::Result<String> {
        let mut state = self.ctx.create_state().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(if language == "auto" { "auto" } else { language }));
        params.set_translate(false);
        params.set_no_context(true);
        params.set_single_segment(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
        params.set_n_threads(threads as i32);

        state.full(params, audio).map_err(|e| anyhow::anyhow!("Ошибка распознавания: {e:?}"))?;

        let mut text = String::new();
        for segment in state.as_iter() {
            text.push_str(&segment.to_string());
        }
        Ok(clean(&text))
    }
}

/// Убираем служебные пометки Whisper вроде [музыка], (смех) и лишние пробелы.
fn clean(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for ch in s.chars() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
