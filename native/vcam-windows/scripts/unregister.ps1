# Unregisters the MeetBurner Camera filter and removes every registry key it created.
param([string]$Dll = "$PSScriptRoot\..\build\Release\MeetBurnerCamera.dll")

$ErrorActionPreference = 'Stop'
$dllPath = (Resolve-Path $Dll).Path
$process = Start-Process -FilePath 'C:\Windows\System32\regsvr32.exe' -ArgumentList '/u', '/s', $dllPath -Wait -PassThru -NoNewWindow
if ($process.ExitCode -ne 0) { throw "regsvr32 /u failed with exit code $($process.ExitCode)" }

$keys = @(
    'HKCU:\Software\Classes\CLSID\{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}',
    'HKCU:\Software\Classes\CLSID\{860BB310-5D01-11D0-BD3B-00A0C911CE86}\Instance\{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}'
)
foreach ($key in $keys) {
    if (Test-Path $key) { throw "registry key still present after unregister: $key" }
}
Write-Host "MeetBurner Camera unregistered"
