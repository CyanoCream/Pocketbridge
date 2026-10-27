# PocketBridge launcher for Windows.
# Opens a separate browser window/profile whose traffic goes through the phone's proxy (port 8888).
#   launch.ps1 -Install -PhoneHost 192.168.43.1   register pocketbridge:// links + shortcuts
#   launch.ps1 "pocketbridge://edge/?host=..."    called by the browser when a logo is clicked
#   launch.ps1 -Browser chrome                    shortcut: finds the phone automatically
#   launch.ps1 -Uninstall
param(
    [string]$Uri = "",
    [string]$Browser = "",
    [string]$PhoneHost = "",
    [switch]$Install,
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$Dir = Join-Path $env:LOCALAPPDATA 'PocketBridge'
$Self = Join-Path $Dir 'launch.ps1'
$LastHostFile = Join-Path $Dir 'last-host.txt'
$WebPort = 8080
$ProxyPort = 8888
$StartUrl = 'https://www.google.com/'

function Show-Message([string]$Text, [string]$Icon = 'Information') {
    [void][System.Windows.Forms.MessageBox]::Show($Text, 'PocketBridge', 'OK', $Icon)
}

$Browsers = [ordered]@{
    chrome  = @{ Name = 'Chrome';  Kind = 'chromium'; Paths = @(
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe",
        "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe") }
    edge    = @{ Name = 'Edge';    Kind = 'chromium'; Paths = @(
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") }
    brave   = @{ Name = 'Brave';   Kind = 'chromium'; Paths = @(
        "$env:ProgramFiles\BraveSoftware\Brave-Browser\Application\brave.exe",
        "${env:ProgramFiles(x86)}\BraveSoftware\Brave-Browser\Application\brave.exe",
        "$env:LOCALAPPDATA\BraveSoftware\Brave-Browser\Application\brave.exe") }
    firefox = @{ Name = 'Firefox'; Kind = 'firefox'; Paths = @(
        "$env:ProgramFiles\Mozilla Firefox\firefox.exe",
        "${env:ProgramFiles(x86)}\Mozilla Firefox\firefox.exe",
        "$env:LOCALAPPDATA\Mozilla Firefox\firefox.exe") }
}

function Find-Exe([string]$Key) {
    foreach ($p in $Browsers[$Key].Paths) { if ($p -and (Test-Path $p)) { return $p } }
    return $null
}

function ConvertTo-UInt32([string]$Ip) {
    $b = ([System.Net.IPAddress]::Parse($Ip)).GetAddressBytes()
    return ([uint32]$b[0] -shl 24) -bor ([uint32]$b[1] -shl 16) -bor ([uint32]$b[2] -shl 8) -bor [uint32]$b[3]
}

# Local IPv4 subnets of this computer as @{ Ip; Prefix } (prefix capped at /24 for scanning).
function Get-LocalSubnets {
    try {
        Get-NetIPAddress -AddressFamily IPv4 -ErrorAction Stop |
            Where-Object { $_.IPAddress -notmatch '^(127\.|169\.254\.)' } |
            ForEach-Object { @{ Ip = $_.IPAddress; Prefix = [int]$_.PrefixLength } }
    } catch { @() }
}

function Test-PrivateIp([string]$Ip) {
    if ($Ip -notmatch '^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$') { return $false }
    $o = 1..4 | ForEach-Object { [int]$Matches[$_] }
    if ($o | Where-Object { $_ -gt 255 }) { return $false }
    return ($o[0] -eq 10) -or ($o[0] -eq 172 -and $o[1] -ge 16 -and $o[1] -le 31) -or ($o[0] -eq 192 -and $o[1] -eq 168)
}

# Accepted: private ranges, or any address inside a subnet this computer is directly connected to
# (some routers use non-private ranges such as 50.50.50.x). A website cannot point us at the internet.
function Test-LocalIp([string]$Ip) {
    if ($Ip -notmatch '^\d{1,3}(\.\d{1,3}){3}$') { return $false }
    try { $target = ConvertTo-UInt32 $Ip } catch { return $false }
    if (Test-PrivateIp $Ip) { return $true }
    foreach ($n in Get-LocalSubnets) {
        if ($n.Prefix -lt 8 -or $n.Prefix -gt 32) { continue }
        $mask = [uint32]([math]::Pow(2, 32) - [math]::Pow(2, 32 - $n.Prefix))
        if (((ConvertTo-UInt32 $n.Ip) -band $mask) -eq ($target -band $mask)) { return $true }
    }
    return $false
}

function Test-Phone([string]$Ip) {
    if (-not (Test-LocalIp $Ip)) { return $false }
    try {
        $r = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "http://${Ip}:$WebPort/pb-ping"
        return ($r.Content.Trim() -eq 'pocketbridge')
    } catch { return $false }
}

function Find-Phone {
    $candidates = New-Object System.Collections.Generic.List[string]
    if (Test-Path $LastHostFile) { $candidates.Add((Get-Content $LastHostFile -Raw).Trim()) }
    # Connected to the phone's hotspot: the phone is the default gateway.
    try {
        Get-NetIPConfiguration | ForEach-Object { $_.IPv4DefaultGateway } | Where-Object { $_ } |
            ForEach-Object { $candidates.Add($_.NextHop) }
    } catch {}
    foreach ($ip in ($candidates | Select-Object -Unique)) { if (Test-Phone $ip) { return $ip } }

    # Same router: quick scan of each local /24 for port 8080.
    $prefixes = Get-LocalSubnets | ForEach-Object { ($_.Ip -split '\.')[0..2] -join '.' } | Select-Object -Unique
    foreach ($prefix in $prefixes) {
        $probes = foreach ($i in 1..254) {
            $c = New-Object System.Net.Sockets.TcpClient
            [pscustomobject]@{ Ip = "$prefix.$i"; Client = $c; Task = $c.ConnectAsync("$prefix.$i", $WebPort) }
        }
        Start-Sleep -Milliseconds 900
        $open = $probes | Where-Object { $_.Task.Status -eq 'RanToCompletion' } | ForEach-Object { $_.Ip }
        $probes | ForEach-Object { $_.Client.Dispose() }
        foreach ($ip in $open) { if (Test-Phone $ip) { return $ip } }
    }
    return $null
}

function Start-Bridge([string]$Key, [string]$Ip) {
    $exe = Find-Exe $Key
    if (-not $exe) { Show-Message "$($Browsers[$Key].Name) tidak ditemukan di komputer ini." 'Warning'; return }
    Set-Content -Path $LastHostFile -Value $Ip -Encoding ascii
    $profileDir = Join-Path $Dir "$Key-profile"
    $proxy = "${Ip}:$ProxyPort"

    if ($Browsers[$Key].Kind -eq 'firefox') {
        New-Item -ItemType Directory -Force -Path $profileDir | Out-Null
        $prefs = @(
            'user_pref("network.proxy.type", 1);',
            "user_pref(`"network.proxy.http`", `"$Ip`");",
            "user_pref(`"network.proxy.http_port`", $ProxyPort);",
            "user_pref(`"network.proxy.ssl`", `"$Ip`");",
            "user_pref(`"network.proxy.ssl_port`", $ProxyPort);",
            'user_pref("network.proxy.share_proxy_settings", true);',
            'user_pref("network.proxy.no_proxies_on", "");',
            'user_pref("network.trr.mode", 5);',
            'user_pref("browser.shell.checkDefaultBrowser", false);',
            'user_pref("browser.aboutwelcome.enabled", false);',
            'user_pref("trailhead.firstrun.didSeeAboutWelcome", true);',
            'user_pref("datareporting.policy.dataSubmissionPolicyBypassNotification", true);',
            'user_pref("browser.startup.homepage_override.mstone", "ignore");'
        )
        Set-Content -Path (Join-Path $profileDir 'user.js') -Value $prefs -Encoding ascii
        Start-Process -FilePath $exe -ArgumentList @('-profile', "`"$profileDir`"", '-no-remote', $StartUrl)
    } else {
        Start-Process -FilePath $exe -ArgumentList @(
            "--user-data-dir=`"$profileDir`"", "--proxy-server=`"http://$proxy`"",
            '--no-first-run', '--no-default-browser-check', $StartUrl)
    }
}

function New-Shortcut([string]$Path, [string]$Key, [string]$Exe) {
    $ws = New-Object -ComObject WScript.Shell
    $s = $ws.CreateShortcut($Path)
    $s.TargetPath = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
    $s.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$Self`" -Browser $Key"
    $s.IconLocation = "$Exe,0"
    $s.Description = "Buka $($Browsers[$Key].Name) dengan internet dari HP (PocketBridge)"
    $s.Save()
}

function Get-ShortcutDirs {
    @([Environment]::GetFolderPath('Desktop'), (Join-Path ([Environment]::GetFolderPath('Programs')) 'PocketBridge'))
}

if ($Install) {
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    $cmd = "`"$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe`" -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$Self`" `"%1`""
    $key = 'HKCU:\Software\Classes\pocketbridge'
    New-Item -Path "$key\shell\open\command" -Force | Out-Null
    Set-ItemProperty -Path $key -Name '(default)' -Value 'URL:PocketBridge Launcher'
    Set-ItemProperty -Path $key -Name 'URL Protocol' -Value ''
    Set-ItemProperty -Path "$key\shell\open\command" -Name '(default)' -Value $cmd

    $made = @()
    foreach ($d in Get-ShortcutDirs) { New-Item -ItemType Directory -Force -Path $d | Out-Null }
    foreach ($k in $Browsers.Keys) {
        $exe = Find-Exe $k
        if (-not $exe) { continue }
        foreach ($d in Get-ShortcutDirs) { New-Shortcut (Join-Path $d "PocketBridge - $($Browsers[$k].Name).lnk") $k $exe }
        $made += $Browsers[$k].Name
    }
    if (Test-LocalIp $PhoneHost) { Set-Content -Path $LastHostFile -Value $PhoneHost -Encoding ascii }
    $list = if ($made) { $made -join ', ' } else { '(tidak ada browser yang dikenali)' }
    Show-Message ("PocketBridge terpasang.`n`nKlik logo browser di halaman Internet, atau pakai shortcut di Desktop / Start Menu.`n`nBrowser ditemukan: $list")
    exit 0
}

if ($Uninstall) {
    Remove-Item -Path 'HKCU:\Software\Classes\pocketbridge' -Recurse -Force -ErrorAction SilentlyContinue
    foreach ($d in Get-ShortcutDirs) {
        Get-ChildItem -Path $d -Filter 'PocketBridge - *.lnk' -ErrorAction SilentlyContinue | Remove-Item -Force
    }
    Remove-Item -Path (Join-Path ([Environment]::GetFolderPath('Programs')) 'PocketBridge') -Recurse -Force -ErrorAction SilentlyContinue
    Show-Message "Link dan shortcut PocketBridge dihapus.`nProfil browser (login tersimpan) masih ada di:`n$Dir"
    exit 0
}

$ip = $null
if ($Uri) {
    if ($Uri -notmatch '^pocketbridge://([a-z]+)') { exit 1 }
    $Browser = $Matches[1]
    if ($Uri -match '[?&]host=([0-9.]+)') { $ip = $Matches[1] }
}
if (-not $Browsers.Contains($Browser)) { Show-Message "Browser tidak dikenal: $Browser" 'Warning'; exit 1 }

if ($ip) {
    if (-not (Test-Phone $ip)) { Show-Message "Tidak bisa menghubungi PocketBridge di $ip.`nPastikan server di HP menyala." 'Warning'; exit 1 }
} else {
    $ip = Find-Phone
    if (-not $ip) { Show-Message "HP dengan PocketBridge tidak ditemukan.`nPastikan server menyala dan laptop terhubung ke WiFi/hotspot yang sama." 'Warning'; exit 1 }
}
Start-Bridge $Browser $ip
