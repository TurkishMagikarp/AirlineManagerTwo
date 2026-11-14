param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-airport-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Isolated explicit paths; each writer exits before its fresh reader starts.
foreach ($version in @('v23', 'v25')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/airport_charges_test.gd -- "--airport-mode=$mode" "--airport-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Airport charges restart failed: $mode" }
    }
}
Write-Output "PASS: v23 frozen tariff migration and v25 player/AI airport continuation in four separate processes; test files: $directory"
