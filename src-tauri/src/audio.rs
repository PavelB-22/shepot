//! Запись с микрофона. Поток записи живёт в отдельном системном потоке,
//! а наружу отдаём только «стоп» и готовый звук 16 кГц моно (так нужно Whisper).
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub const TARGET_RATE: u32 = 16_000;
/// Максимум 2 минуты записи
const MAX_SECONDS: usize = 120;

pub struct Recorder {
    stop_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    samples: Arc<Mutex<Vec<f32>>>,
    rate: u32,
}

impl Recorder {
    /// Начать запись с микрофона по умолчанию.
    pub fn start() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("Микрофон не найден"))?;
        let config = device.default_input_config()?;
        let rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();

        let samples = Arc::new(Mutex::new(Vec::<f32>::with_capacity(rate as usize * 10)));
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let buf = samples.clone();
        let max_len = rate as usize * MAX_SECONDS;

        let handle = std::thread::spawn(move || {
            let err_fn = |e| eprintln!("ошибка микрофона: {e}");
            // Складываем каналы в моно и копим звук
            let push = move |data: &[f32]| {
                let mut b = buf.lock().unwrap();
                if b.len() >= max_len {
                    return;
                }
                for frame in data.chunks(channels) {
                    b.push(frame.iter().sum::<f32>() / channels as f32);
                }
            };
            let stream = match format {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &stream_config,
                    move |d: &[f32], _| push(d),
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &stream_config,
                    move |d: &[i16], _| {
                        let v: Vec<f32> = d.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                        push(&v)
                    },
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::U16 => device.build_input_stream(
                    &stream_config,
                    move |d: &[u16], _| {
                        let v: Vec<f32> = d.iter().map(|s| (*s as f32 - 32768.0) / 32768.0).collect();
                        push(&v)
                    },
                    err_fn,
                    None,
                ),
                other => {
                    let _ = ready_tx.send(Err(format!("Формат звука {other:?} не поддерживается")));
                    return;
                }
            };
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    let _ = ready_tx.send(Err(format!("Нет доступа к микрофону: {e}")));
                    return;
                }
            };
            if let Err(e) = stream.play() {
                let _ = ready_tx.send(Err(format!("Не удалось включить микрофон: {e}")));
                return;
            }
            let _ = ready_tx.send(Ok(()));
            let _ = stop_rx.recv(); // ждём команду «стоп»
            drop(stream);
        });

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { stop_tx, handle: Some(handle), samples, rate }),
            Ok(Err(e)) => Err(anyhow::anyhow!(e)),
            Err(_) => Err(anyhow::anyhow!("Микрофон не запустился")),
        }
    }

    /// Звук, записанный на этот момент (запись продолжается). 16 кГц моно.
    pub fn snapshot(&self) -> Vec<f32> {
        let raw = self.samples.lock().unwrap().clone();
        resample(&raw, self.rate, TARGET_RATE)
    }

    /// Остановить запись и получить звук 16 кГц моно.
    pub fn stop(mut self) -> Vec<f32> {
        let _ = self.stop_tx.send(());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        let raw = std::mem::take(&mut *self.samples.lock().unwrap());
        resample(&raw, self.rate, TARGET_RATE)
    }
}

/// Простая линейная передискретизация (для речи этого достаточно).
fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            let a = input[idx];
            let b = *input.get(idx + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

/// Есть ли в записи речь (а не тишина)?
pub fn has_voice(samples: &[f32]) -> bool {
    if samples.len() < (TARGET_RATE as usize * 3) / 10 {
        return false; // короче 0,3 с
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    rms > 0.004
}
