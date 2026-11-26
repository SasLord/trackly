<#
  diag.ps1 - диагностика окружения для пилота Trackly Link (DST-06).

  Зачем: собрать факты, нужные для решения «установка на пользователя
  работает в этом домене или нет» (WebView2, AppLocker/SRP, путь, разрядность, RDS).

  Чего НЕ делает: ничего не меняет в системе, не требует прав администратора,
  не пишет в реестр и не удаляет файлы. Единственная запись - JSON-файл итога
  во временном каталоге пользователя.

  Что выводит: только обезличиваемые поля. Имя компьютера, имя пользователя,
  адреса, домен и Subject сертификата в файл итога не попадают никогда.
  Subject подписи показывается только на экране с пометкой «не пересылать».

  Запуск: powershell -ExecutionPolicy Bypass -File diag.ps1 [-NoLaunch]
  -NoLaunch пропускает пробный запуск trackly-link.exe.
#>
[CmdletBinding()]
param([switch]$NoLaunch)

$ErrorActionPreference = 'Continue'
$WebView2Guid = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$result = [ordered]@{}

# --- ОС и разрядность ---
$os = Get-CimInstance Win32_OperatingSystem
$result.os = [ordered]@{
  caption     = $os.Caption
  build       = $os.BuildNumber
  is64bitOS   = [Environment]::Is64BitOperatingSystem
  productType = [int]$os.ProductType   # 1 = рабочая станция, 3 = сервер
  isServer    = ([int]$os.ProductType -eq 3)
}

# --- WebView2 Runtime ---
function Get-Wv2Version($path) {
  try { (Get-ItemProperty -Path $path -Name pv -ErrorAction Stop).pv } catch { $null }
}
$pvMachine = Get-Wv2Version "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$WebView2Guid"
$pvUser    = Get-Wv2Version "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$WebView2Guid"
function Test-Wv2($v) { $v -and ($v -ne '0.0.0.0') }
$result.webview2 = [ordered]@{
  machineVersion = $pvMachine
  userVersion    = $pvUser
  installed      = [bool]((Test-Wv2 $pvMachine) -or (Test-Wv2 $pvUser))
}

# --- Установка Link ---
$candidateDirs = @(
  (Join-Path $env:LOCALAPPDATA 'Programs\Trackly Link'),
  (Join-Path $env:LOCALAPPDATA 'Trackly Link')
)
$found = @()
foreach ($d in $candidateDirs) {
  $exe = Join-Path $d 'trackly-link.exe'
  $found += [ordered]@{
    pathKind = if ($d -like '*\Programs\*') { 'LOCALAPPDATA\Programs\Trackly Link' } else { 'LOCALAPPDATA\Trackly Link' }
    exists   = (Test-Path $exe)
  }
}
$uninstKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Trackly Link'
$uninst = $null
try { $uninst = Get-ItemProperty -Path $uninstKey -ErrorAction Stop } catch {}

$exePath = $null
foreach ($d in $candidateDirs) {
  $p = Join-Path $d 'trackly-link.exe'
  if (Test-Path $p) { $exePath = $p; break }
}

function Get-PeMachine($file) {
  try {
    $fs = [IO.File]::OpenRead($file)
    try {
      $br = New-Object IO.BinaryReader($fs)
      $fs.Position = 0x3C
      $peOffset = $br.ReadInt32()
      $fs.Position = $peOffset + 4
      return [int]$br.ReadUInt16()
    } finally { $fs.Dispose() }
  } catch { return $null }
}

$machine = $null; $isX64 = $null; $sigStatus = $null
if ($exePath) {
  $machine = Get-PeMachine $exePath
  $isX64 = ($machine -eq 0x8664)
  $sig = Get-AuthenticodeSignature -FilePath $exePath
  $sigStatus = [string]$sig.Status
  if ($sig.SignerCertificate) {
    Write-Host ("Subject подписи (на экране, НЕ пересылать): " + $sig.SignerCertificate.Subject) -ForegroundColor Yellow
  }
}
$result.install = [ordered]@{
  candidates         = $found
  uninstallEntry     = [bool]$uninst
  displayVersion     = if ($uninst) { $uninst.DisplayVersion } else { $null }
  peMachineHex       = if ($machine) { ('0x{0:X4}' -f $machine) } else { $null }
  isX64              = $isX64
  signatureStatus    = $sigStatus
  signed             = [bool]($sigStatus -and $sigStatus -ne 'NotSigned')
}

# --- AppLocker ---
function Get-ProductEvents($log, $since) {
  try {
    $ev = Get-WinEvent -FilterHashtable @{ LogName = $log; StartTime = $since } -ErrorAction Stop |
      Where-Object { $_.Message -like '*trackly-link*' }
    $ev | Group-Object Id | ForEach-Object { [ordered]@{ id = [int]$_.Name; count = $_.Count } }
  } catch { @() }
}
function Get-AppLockerSummary {
  $out = [ordered]@{}
  try {
    $xml = [xml](Get-AppLockerPolicy -Effective -Xml -ErrorAction Stop)
    $cols = @()
    foreach ($c in $xml.AppLockerPolicy.RuleCollection) {
      $n = 0
      foreach ($t in 'FilePathRule','FilePublisherRule','FileHashRule') {
        $n += @($c.$t).Where({ $_ }).Count
      }
      $cols += [ordered]@{ collection = $c.Type; enforcementMode = $c.EnforcementMode; rules = $n }
    }
    $out.collections = $cols
  } catch { $out.collections = @(); $out.note = 'политика недоступна или не задана' }
  $out
}
$since = (Get-Date).AddDays(-7)
$result.appLocker = [ordered]@{
  policy = Get-AppLockerSummary
  # 8002 разрешено, 8003 audit-разрешено, 8004 заблокировано, 8005-8007 - прочие коллекции
  exeEvents    = Get-ProductEvents 'Microsoft-Windows-AppLocker/EXE and DLL' $since
  scriptEvents = Get-ProductEvents 'Microsoft-Windows-AppLocker/MSI and Script' $since
}

# --- SRP ---
$srp = $null
try { $srp = Get-ItemProperty 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\Safer\CodeIdentifiers' -ErrorAction Stop } catch {}
$srpEvents = @()
try {
  $srpEvents = Get-WinEvent -FilterHashtable @{ ProviderName = 'Microsoft-Windows-SoftwareRestrictionPolicies'; StartTime = $since } -ErrorAction Stop |
    Group-Object Id | ForEach-Object { [ordered]@{ id = [int]$_.Name; count = $_.Count } }
} catch {}
$result.srp = [ordered]@{
  configured   = [bool]$srp
  defaultLevel = if ($srp) { $srp.DefaultLevel } else { $null }
  policyScope  = if ($srp) { $srp.PolicyScope } else { $null }
  events       = $srpEvents
}

# --- Сеанс ---
$sessionType = if ($env:SESSIONNAME -like 'RDP-*') { 'RDP' } elseif ($env:SESSIONNAME -eq 'Console') { 'Console' } else { 'Other' }
$activeCount = $null
try {
  $q = & quser 2>$null
  if ($q) { $activeCount = @($q).Count - 1 }   # без заголовка; имена не сохраняются
} catch {}
$result.session = [ordered]@{
  type               = $sessionType
  isServer           = $result.os.isServer
  sessionsCount      = $activeCount
}

# --- Пробный запуск ---
$launch = [ordered]@{ attempted = $false }
if (-not $NoLaunch -and $exePath) {
  $launch.attempted = $true
  $t0 = Get-Date
  try {
    $p = Start-Process -FilePath $exePath -PassThru -ErrorAction Stop
    Start-Sleep -Seconds 5
    $launch.hasExited = $p.HasExited
    $launch.exitCode  = if ($p.HasExited) { $p.ExitCode } else { $null }
    if (-not $p.HasExited) { try { $p.CloseMainWindow() | Out-Null; Start-Sleep -Seconds 1; if (-not $p.HasExited) { $p.Kill() } } catch {} }
    $launch.startError = $null
  } catch {
    $launch.startError = 'запуск отклонён системой'
  }
  $recent = $t0.AddMinutes(-1)
  $launch.appLockerEvents = Get-ProductEvents 'Microsoft-Windows-AppLocker/EXE and DLL' $recent
}
$result.launch = $launch

# --- Итог ---
$outFile = Join-Path $env:TEMP ('trackly-link-diag-{0}.json' -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
$result | ConvertTo-Json -Depth 8 | Set-Content -Path $outFile -Encoding UTF8
Write-Host "Итог (обезличенный): $outFile"
if ($isX64 -eq $false) { Write-Warning 'Установленный exe не x64 (ожидается PE Machine 0x8664).' }
