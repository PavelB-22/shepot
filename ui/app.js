// Ссылка на Telegram-канал — поменяй на свою
const CHANNEL_URL = 'https://t.me/';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);

let state = null;
const downloading = {}; // id -> процент

const isMac = () => state && state.platform === 'macos';
function hotkeyLabel(h) {
  const mac = isMac();
  return h.replace('CommandOrControl', mac ? '⌘' : 'Ctrl')
    .replace('Shift', mac ? '⇧' : 'Shift').replace('Alt', mac ? '⌥' : 'Alt')
    .replace('Space', mac ? 'Пробел' : 'Пробел').split('+').join(mac ? '' : '+');
}
const HOTKEYS = ['CommandOrControl+Shift+Space', 'Alt+Space', 'CommandOrControl+Alt+Space', 'F8', 'F9'];

async function load() {
  state = await invoke('get_state');
  render();
}

function render() {
  const s = state.settings;
  document.querySelectorAll('.mac-only').forEach((el) => (el.hidden = !isMac()));
  document.querySelectorAll('.hk').forEach((el) => (el.textContent = hotkeyLabel(s.hotkey)));
  $('onboarding').hidden = s.onboarded;
  $('main').hidden = !s.onboarded;

  $('hotkey').innerHTML = HOTKEYS.map((h) => `<option value="${h}">${hotkeyLabel(h)}</option>`).join('');
  $('hotkey').value = s.hotkey;
  $('mode').value = s.mode;
  $('language').value = s.language;
  $('autostart').checked = s.autostart;
  $('live').checked = s.live;
  $('autoUpdate').checked = s.auto_update;
  $('ver').textContent = state.version;
  $('liveHint').textContent = !s.live ? '' : s.mode === 'toggle'
    ? 'Слова появляются прямо в поле, пока говоришь'
    : 'Слова бегут в подсказке внизу экрана. Чтобы они печатались прямо в поле — выбери «Нажал — начал, нажал — закончил»';

  renderModels($('models'));
  renderModels($('onbModels'));
  $('finishOnb').disabled = !state.models.find((m) => m.id === s.model && m.downloaded);
  renderHistory(state.history);
}

function renderModels(box) {
  box.innerHTML = '';
  for (const m of state.models) {
    const el = document.createElement('div');
    el.className = 'model' + (m.id === state.settings.model ? ' selected' : '');
    const pct = downloading[m.id];
    let act = m.downloaded ? (m.id === state.settings.model ? '✓ Выбрано' : 'Выбрать') : `Скачать · ${m.size_mb} МБ`;
    if (pct !== undefined) act = `${pct}%`;
    el.innerHTML = `<div class="info"><b>${m.title}</b><small>${m.note}</small>
      ${pct !== undefined ? `<div class="progress"><div style="width:${pct}%"></div></div>` : ''}</div>
      <span class="act">${act}</span>`;
    el.onclick = () => pickModel(m);
    box.appendChild(el);
  }
}

async function pickModel(m) {
  if (downloading[m.id] !== undefined) return;
  if (!m.downloaded) {
    downloading[m.id] = 0;
    render();
    try {
      await invoke('download_model', { id: m.id });
    } catch (e) {
      $('err').textContent = e;
      alert(e);
    }
    delete downloading[m.id];
    state = await invoke('get_state');
  }
  if (state.models.find((x) => x.id === m.id)?.downloaded) {
    await save({ model: m.id });
  }
  render();
}

async function save(patch) {
  const next = { ...state.settings, ...patch };
  $('err').textContent = '';
  try {
    await invoke('save_settings', { new: next });
    state.settings = next;
  } catch (e) {
    $('err').textContent = e;
    state = await invoke('get_state');
  }
  render();
}

function renderHistory(list) {
  $('histEmpty').hidden = list.length > 0;
  $('history').innerHTML = '';
  for (const t of list) {
    const li = document.createElement('li');
    li.innerHTML = '<span></span><button title="Скопировать">📋</button>';
    li.querySelector('span').textContent = t;
    li.querySelector('button').onclick = () => invoke('copy_text', { text: t });
    $('history').appendChild(li);
  }
}

$('hotkey').onchange = (e) => save({ hotkey: e.target.value });
$('mode').onchange = (e) => save({ mode: e.target.value });
$('language').onchange = (e) => save({ language: e.target.value });
$('autoUpdate').onchange = (e) => save({ auto_update: e.target.checked });
$('checkUpd').onclick = async (e) => {
  e.preventDefault();
  $('updMsg').textContent = '— проверяю…';
  try {
    const v = await invoke('update_now');
    $('updMsg').textContent = v ? `— ставлю ${v}, программа перезапустится` : '— у тебя последняя версия ✓';
  } catch (err) { $('updMsg').textContent = '— ' + err; }
};
listen('update', (e) => { $('updMsg').textContent = `— скачиваю версию ${e.payload.version}…`; });
$('live').onchange = (e) => save({ live: e.target.checked });
$('autostart').onchange = (e) => save({ autostart: e.target.checked });
$('finishOnb').onclick = () => save({ onboarded: true });
$('channel').onclick = (e) => { e.preventDefault(); window.__TAURI__.opener.openUrl(CHANNEL_URL); };
document.querySelectorAll('nav button').forEach((b) => {
  b.onclick = () => {
    document.querySelectorAll('nav button').forEach((x) => x.classList.toggle('active', x === b));
    document.querySelectorAll('.tab').forEach((t) => (t.hidden = t.id !== 'tab-' + b.dataset.tab));
  };
});

listen('download-progress', (e) => {
  const { id, done, total } = e.payload;
  downloading[id] = Math.min(100, Math.round((done / total) * 100));
  render();
});
listen('history', (e) => { state.history = e.payload; renderHistory(e.payload); });

load();
