<#
  install-silent.ps1 - имитация вызова установщика из скрипта входа GPO (пилот DST-06).

  Работает от обычного пользователя: пишет только в профиль и HKCU.
  Не обходит AppLocker/SRP - если запуск заблокирован, это результат пилота (D-04).

  Примеры:
    .\install-silent.ps1 -Installer .\<имя-установщика>.exe
    .\install-silent.ps1 -Uninstall
#>
[CmdletBinding()]
param(
  [string]$Installer,
  [switch]$Uninstall
)
$ErrorActionPreference = 'Stop'

$target    = Join-Path $env:LOCALAPPDATA 'Programs\Trackly Link'
$exe       = Join-Path $target 'trackly-link.exe'
$uninstKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Trackly Link'
$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'

$fail = 0
function Check($ok, $msg) {
  if ($ok) { Write-Host "[ OK ] $msg" } else { Write-Host "[FAIL] $msg" -ForegroundColor Red; $script:fail++ }
}
function Find-Shortcut {
  Get-ChildItem -Path $startMenu -Recurse -Filter '*Trackly Link*.lnk' -ErrorAction SilentlyContinue | Select-Object -First 1
}

if ($Uninstall) {
  $loc = $target
  try { $p = Get-ItemProperty $uninstKey -ErrorAction Stop; if ($p.InstallLocation) { $loc = $p.InstallLocation } } catch {}
  $un = Join-Path $loc 'uninstall.exe'
  if (-not (Test-Path $un)) { Write-Host "uninstall.exe не найден - нечего удалять"; exit 1 }
  $proc = Start-Process -FilePath $un -ArgumentList '/S' -Wait -PassThru
  Start-Sleep -Seconds 2
  Check ($proc.ExitCode -eq 0) "код возврата деинсталлятора = 0 (получено $($proc.ExitCode))"
  Check (-not (Test-Path $loc)) 'каталог установки удалён'
  Check (-not (Find-Shortcut)) 'ярлык в меню «Пуск» удалён'
  Check (-not (Test-Path $uninstKey)) 'запись HKCU Uninstall удалена'
  exit $fail
}

if (-not $Installer) { throw 'Укажите -Installer <путь к установщику>' }
if (-not (Test-Path $Installer)) { throw 'Установщик не найден' }

if ((Test-Path $exe) -and (Test-Path $uninstKey)) {
  Write-Host 'Уже установлено - повторный запуск установщика поверх (идемпотентность).'
}

# /D= обязан идти последним и без кавычек (особенность NSIS)
$proc = Start-Process -FilePath $Installer -ArgumentList '/S', "/D=$target" -Wait -PassThru
Check ($proc.ExitCode -eq 0) "код возврата установщика = 0 (получено $($proc.ExitCode))"
Check (Test-Path $exe) 'trackly-link.exe в целевом каталоге (Programs\Trackly Link)'
Check (Test-Path $uninstKey) 'запись HKCU Uninstall создана'
Check ([bool](Find-Shortcut)) 'ярлык в меню «Пуск»'
exit $fail
