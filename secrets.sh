#!/bin/bash
# Помощник: по очереди кладёт каждый секрет в буфер обмена,
# а ты вставляешь его на странице GitHub.  Запуск: bash secrets.sh
cd "$(dirname "$0")"
S=.secret
URL="https://github.com/PavelB-22/shepot/settings/secrets/actions"

step() {  # $1 — имя секрета, $2 — файл
  printf "%s" "$(cat "$S/$2")" | pbcopy
  echo
  echo "────────────────────────────────────────────"
  echo "  Секрет:  $1"
  echo "────────────────────────────────────────────"
  echo "  ✅ Значение уже в буфере обмена."
  echo "  На GitHub: если секрет $1 уже есть — нажми ✏️ (карандаш) рядом с ним,"
  echo "  если нет — «New repository secret», в Name впиши:  $1"
  echo "  В поле Secret нажми ⌘V и затем «Update secret» / «Add secret»."
  read -p "  Готово? Нажми Enter, чтобы перейти к следующему... "
}

open "$URL"
echo "Открыл страницу секретов на GitHub."
step MACOS_CERT_P12            shepot-cert.p12.b64
step MACOS_CERT_PASSWORD       shepot-cert-password.txt
step TAURI_SIGNING_PRIVATE_KEY updater.key
printf "" | pbcopy
echo
echo "🎉 Все три секрета обновлены. Буфер очищен."
echo "Теперь выпусти версию:  bash publish.sh"
