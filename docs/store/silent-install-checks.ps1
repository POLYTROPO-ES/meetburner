# Microsoft Store: silent install check

Runs the Store installer silently and checks the per-user camera keys on a test VM. Do not run it
on a machine whose MeetBurner installation you want to keep: it installs, upgrades and uninstalls
the app.

param(
    [Parameter(Mandatory = $true)][string]$Installer,
    [string]$NewerInstaller,
    [switch]$SkipUninstall
)

$ErrorActionPreference = 'Stop'
$directShow = 'HKCU:\Software\Classes\CLSID\{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}'
$mediaFoundation = 'HKCU:\Software\Classes\CLSID\{9D6A2B41-3C57-4A0E-8F2D-7B1E5C3A9F64}'
$installDir = Join-Path $env:LOCALAPPDATA 'MeetBurner'

function Invoke-Silent([string]$Path) {
    Write-Host "Running $(Split-Path $Path -Leaf) /S"
    $process = Start-Process -FilePath $Path -ArgumentList '/S' -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "$(Split-Path $Path -Leaf) exited with $($process.ExitCode)" }
}

Invoke-Silent (Resolve-Path $Installer).Path
if (-not (Test-Path (Join-Path $installDir 'meetburner.exe'))) { throw 'meetburner.exe is missing after the install' }
Write-Host 'Install OK'

if ($NewerInstaller) {
    Invoke-Silent (Resolve-Path $NewerInstaller).Path
    if (-not (Test-Path (Join-Path $installDir 'meetburner.exe'))) { throw 'meetburner.exe is missing after the upgrade' }
    Write-Host 'Upgrade OK (check the settings and the camera registration by hand)'
}

Write-Host 'Register the cameras from the app before the uninstall check.'
Write-Host "DirectShow key: $(Test-Path $directShow)"
Write-Host "MF key:         $(Test-Path $mediaFoundation)"

if (-not $SkipUninstall) {
    $uninstaller = Join-Path $installDir 'uninstall.exe'
    if (-not (Test-Path $uninstaller)) { throw "uninstaller not found: $uninstaller" }
    $process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "the uninstaller exited with $($process.ExitCode)" }
    Write-Host "DirectShow key after uninstall: $(Test-Path $directShow)"
    Write-Host "MF key after uninstall:         $(Test-Path $mediaFoundation)"
    Write-Host 'Uninstall OK: both keys must be False for the account that ran it.'
}
