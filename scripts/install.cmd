@echo off
rem Install tempo-cli for the current user on Windows.
rem Copies tempo-cli.exe to %LOCALAPPDATA%\Programs\tempo and adds that folder to your PATH.
rem The PowerShell code at the end of this file does the work.
set "TEMPO_SCRIPT=%~f0"
powershell -NoProfile -ExecutionPolicy Bypass -Command "$s = [IO.File]::ReadAllText($env:TEMPO_SCRIPT); Invoke-Expression $s.Substring($s.IndexOf([char]10 + '#' + 'PS'))"
set "TEMPO_EXIT=%ERRORLEVEL%"
echo.
pause
exit /b %TEMPO_EXIT%

#PS
$ErrorActionPreference = 'Stop'
$src = Join-Path (Split-Path $env:TEMPO_SCRIPT) 'tempo-cli.exe'
$dest = Join-Path $env:LOCALAPPDATA 'Programs\tempo'

if (-not (Test-Path $src)) {
    Write-Host 'error: tempo-cli.exe is not in the same folder as install.cmd.' -ForegroundColor Red
    Write-Host 'Extract the whole zip file, then run install.cmd again.'
    exit 1
}

try {
    New-Item -ItemType Directory -Force $dest | Out-Null
    Copy-Item $src (Join-Path $dest 'tempo-cli.exe') -Force
    Unblock-File (Join-Path $dest 'tempo-cli.exe')
} catch {
    Write-Host "error: could not copy tempo-cli.exe: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host 'If tempo-cli is running, stop it with Ctrl+C and run install.cmd again.'
    exit 1
}
Write-Host "Installed tempo-cli to $dest"

# Read the raw user PATH, so that entries like %USERPROFILE%\bin stay unexpanded.
$key = Get-Item 'HKCU:\Environment'
$path = $key.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
$parts = @($path -split ';' | Where-Object { $_ })
if ($parts -contains $dest) {
    Write-Host 'Done. Open a new terminal and type: tempo-cli --help'
    exit 0
}

New-ItemProperty 'HKCU:\Environment' -Name Path -Value (($parts + $dest) -join ';') -PropertyType ExpandString -Force | Out-Null
# Tell Windows that the environment changed, so that new terminals see the new PATH.
[Environment]::SetEnvironmentVariable('TEMPO_INSTALL', '1', 'User')
[Environment]::SetEnvironmentVariable('TEMPO_INSTALL', $null, 'User')

Write-Host "Added $dest to your PATH."
Write-Host 'Done. Open a new terminal and type: tempo-cli --help'
