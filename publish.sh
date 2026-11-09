#!/bin/bash
# Выпустить новую версию Shepot:  bash publish.sh
# Сам поднимает номер версии, отправляет код на GitHub и запускает сборку.
# Программы у пользователей обновятся сами в течение нескольких часов.
set -e
cd "$(dirname "$0")"

mkdir -p .github/workflows
cp ci/release.yml .github/workflows/release.yml

# Текущая версия и следующая (0.1.4 → 0.1.5)
CUR=$(grep -m1 '"version"' src-tauri/tauri.conf.json | sed -E 's/.*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')
[ -d .git ] || git init -b main
if git rev-parse "v$CUR" >/dev/null 2>&1; then
  NEXT=$(echo "$CUR" | awk -F. '{print $1"."$2"."$3+1}')
  echo "▶ Версия $CUR уже выпущена — поднимаю до $NEXT"
  perl -0pi -e "s/\"version\": \"$CUR\"/\"version\": \"$NEXT\"/" src-tauri/tauri.conf.json package.json
  perl -0pi -e "s/^version = \"$CUR\"/version = \"$NEXT\"/m" src-tauri/Cargo.toml
  perl -0pi -e "s/(name = \"shepot\"\nversion = )\"$CUR\"/\${1}\"$NEXT\"/" src-tauri/Cargo.lock
  perl -0pi -e "s/\"version\": \"$CUR\"/\"version\": \"$NEXT\"/g" package-lock.json
  CUR=$NEXT
fi

git add -A
git commit -m "Shepot $CUR" || echo "(нет новых изменений)"

if ! git remote | grep -q origin; then
  read -p "Твой логин на GitHub: " LOGIN
  git remote add origin "https://github.com/$LOGIN/shepot.git"
fi

echo "▶ Отправляю код на GitHub..."
git push -u origin main
git tag "v$CUR"
git push origin "v$CUR"
echo "✅ Версия $CUR отправлена. Через ~15 минут она появится тут:"
echo "   $(git remote get-url origin | sed 's/\.git$//')/releases"
echo "   Установленные программы обновятся сами."
