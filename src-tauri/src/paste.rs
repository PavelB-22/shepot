//! Вставка текста туда, где стоит курсор: кладём в буфер и жмём Ctrl/Cmd+V,
//! потом возвращаем буфер как был.
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread::sleep;
use std::time::Duration;

pub fn paste_text(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    let previous = clipboard.get_text().ok();
    clipboard.set_text(text.to_string())?;
    sleep(Duration::from_millis(60));

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    // Клавиша V по её физическому коду — чтобы работало и на русской раскладке
    #[cfg(target_os = "macos")]
    let (modifier, v_key) = (Key::Meta, Key::Other(9)); // kVK_ANSI_V
    #[cfg(target_os = "windows")]
    let (modifier, v_key) = (Key::Control, Key::Other(0x56)); // VK_V
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let (modifier, v_key) = (Key::Control, Key::Unicode('v'));

    enigo.key(modifier, Direction::Press).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    enigo.key(v_key, Direction::Click).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    enigo.key(modifier, Direction::Release).map_err(|e| anyhow::anyhow!("{e:?}"))?;

    // Даём программе время вставить, затем возвращаем старый буфер
    sleep(Duration::from_millis(400));
    if let Some(prev) = previous {
        let _ = clipboard.set_text(prev);
    }
    Ok(())
}

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    arboard::Clipboard::new()?.set_text(text.to_string())?;
    Ok(())
}

/// Напечатать текст «как с клавиатуры» — для живого режима, кусочками.
pub fn type_text(text: &str) -> anyhow::Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    enigo.text(text).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    Ok(())
}
