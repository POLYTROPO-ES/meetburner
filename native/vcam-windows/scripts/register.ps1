# Registers the MeetBurner Camera filter for the current user (HKCU, no UAC).
param([string]$Dll = "$PSScriptRoot\..\build\Release\MeetBurnerCamera.dll")

$ErrorActionPreference = 'Stop'
$dllPath = (Resolve-Path $Dll).Path
$process = Start-Process -FilePath 'C:\Windows\System32\regsvr32.exe' -ArgumentList '/s', $dllPath -Wait -PassThru -NoNewWindow
if ($process.ExitCode -ne 0) { throw "regsvr32 failed with exit code $($process.ExitCode)" }

$key = 'HKCU:\Software\Classes\CLSID\{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}\InprocServer32'
if (-not (Test-Path $key)) { throw "registration key not found: $key" }
Write-Host "MeetBurner Camera registered from $dllPath"
