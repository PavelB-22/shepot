# Shepot — голос в текст, бесплатно и без интернета

Зажал клавишу → сказал → отпустил → текст появился там, где курсор. В Telegram, браузере, Word, где угодно.
Распознавание работает прямо на компьютере (модель Whisper), голос никуда не отправляется.

**Mac** (Apple Silicon) и **Windows 10/11**.

## Установка
**Mac**
1. Скачай `Shepot_..._aarch64.dmg` со страницы Releases, открой, перетащи Shepot в «Программы».
2. В Терминале: `xattr -dr com.apple.quarantine /Applications/Shepot.app`
3. Запусти. Разреши доступ к микрофону и «Универсальному доступу» (нужен, чтобы вставлять текст).

**Windows**
1. Скачай `Shepot_..._x64-setup.exe`, запусти.
2. Если Windows скажет «защитила компьютер» — «Подробнее» → «Выполнить в любом случае».

## Как пользоваться
- При первом запуске выбери качество — модель скачается один раз.
- Зажми **Ctrl+Shift+Пробел** (на Маке **⌘⇧Пробел**), говори, отпусти.
- Настройки и история — через значок в трее / строке меню.

## Для разработчика
```
npm install
npm run tauri dev      # запуск в режиме разработки
npm run tauri build    # сборка установщика
```
Релиз: `git tag v0.1.0 && git push origin v0.1.0` → GitHub Actions соберёт Mac и Windows.

## Публикация (для Павла)
1. На github.com → **New repository** → имя `shepot`, Public, ничего не добавлять → Create.
2. Токен: github.com → Settings → Developer settings → Personal access tokens → **Tokens (classic)** → Generate new token → галочки **repo** и **workflow** → скопировать.
3. В Терминале: `cd ~/Projects/Shepot && bash publish.sh` — логин, потом токен вместо пароля.
4. Через ~15 минут на странице Releases появятся `.dmg` для Mac и `.exe` для Windows.
