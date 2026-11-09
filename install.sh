#!/bin/bash
# Установка / обновление Shepot на Mac одной командой:
#   curl -fsSL https://raw.githubusercontent.com/PavelB-22/shepot/main/install.sh | bash
set -e
REPO="PavelB-22/shepot"

if [ "$(uname -m)" != "arm64" ]; then
  echo "❌ Пока Shepot работает только на Маках с Apple Silicon (M1 и новее)."
  exit 1
fi

echo "▶ Ищу последнюю версию Shepot..."
URL=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep -o '"browser_download_url": *"[^"]*aarch64\.dmg"' | head -1 | cut -d'"' -f4)
[ -z "$URL" ] && { echo "❌ Не нашёл файл для Mac. Попробуй позже."; exit 1; }

TMP=$(mktemp -d)
echo "▶ Скачиваю $(basename "$URL")..."
curl -fL --progress-bar -o "$TMP/Shepot.dmg" "$URL"

echo "▶ Устанавливаю в «Программы»..."
hdiutil attach -nobrowse -quiet -mountpoint "$TMP/mnt" "$TMP/Shepot.dmg"
pkill -x Shepot 2>/dev/null || true
sleep 1
rm -rf /Applications/Shepot.app
cp -R "$TMP/mnt/Shepot.app" /Applications/
hdiutil detach -quiet "$TMP/mnt"
xattr -dr com.apple.quarantine /Applications/Shepot.app 2>/dev/null || true
rm -rf "$TMP"

open /Applications/Shepot.app
echo "✅ Готово! Shepot запущен. Дальше он будет обновляться сам."
