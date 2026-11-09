# Установка / обновление Shepot на Windows одной командой (PowerShell):
#   irm https://raw.githubusercontent.com/PavelB-22/shepot/main/install.ps1 | iex
$ErrorActionPreference = 'Stop'
$repo = 'PavelB-22/shepot'
Write-Host '> Ищу последнюю версию Shepot...'
$rel = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$asset = $rel.assets | Where-Object { $_.name -like '*x64-setup.exe' } | Select-Object -First 1
if (-not $asset) { Write-Host 'Не нашёл установщик для Windows. Попробуй позже.'; return }
$file = Join-Path $env:TEMP $asset.name
Write-Host "> Скачиваю $($asset.name)..."
Invoke-WebRequest $asset.browser_download_url -OutFile $file -UseBasicParsing
Get-Process Shepot -ErrorAction SilentlyContinue | Stop-Process -Force
Write-Host '> Устанавливаю...'
Start-Process $file -ArgumentList '/S' -Wait
Remove-Item $file -ErrorAction SilentlyContinue
$exe = Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Shepot') -Filter *.exe -ErrorAction SilentlyContinue |
  Where-Object { $_.Name -notlike 'uninstall*' } | Select-Object -First 1
if ($exe) { Start-Process $exe.FullName }
Write-Host 'Готово! Shepot установлен и дальше будет обновляться сам.'
