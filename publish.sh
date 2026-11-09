#!/bin/bash
# Отправляет Shepot на GitHub и запускает сборку установщиков для Mac и Windows.
#   bash publish.sh            — первая публикация / новая версия
set -e
cd "$(dirname "$0")"

# Файл автосборки кладём на место (папка .github скрыта в Finder)
mkdir -p .github/workflows
cp ci/release.yml .github/workflows/release.yml

[ -d .git ] || git init -b main
git add -A
git commit -m "Shepot $(date '+%Y-%m-%d %H:%M')" || echo "(нет новых изменений)"

if ! git remote | grep -q origin; then
  read -p "Твой логин на GitHub: " LOGIN
  git remote add origin "https://github.com/$LOGIN/shepot.git"
fi

echo "▶ Отправляю код на GitHub..."
echo "  (если спросит пароль — вставь токен GitHub, не обычный пароль)"
git push -u origin main

VERSION=$(node -p "require('./src-tauri/tauri.conf.json').version" 2>/dev/null || grep -m1 '"version"' src-tauri/tauri.conf.json | sed 's/[^0-9.]//g')
if git rev-parse "v$VERSION" >/dev/null 2>&1; then
  echo "⚠️  Версия v$VERSION уже опубликована. Чтобы выпустить новую — подними \"version\" в src-tauri/tauri.conf.json"
else
  git tag "v$VERSION"
  git push origin "v$VERSION"
  echo "✅ Готово! Сборка запущена. Через ~15 минут установщики появятся тут:"
  echo "   $(git remote get-url origin | sed 's/\.git$//')/releases"
fi
