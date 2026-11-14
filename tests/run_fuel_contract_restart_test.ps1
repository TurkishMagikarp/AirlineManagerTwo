param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-fuel-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Isolated explicit paths; each writer exits before its fresh reader starts.
foreach ($version in @('v20', 'v21')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/fuel_contract_test.gd -- "--fuel-mode=$mode" "--fuel-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Fuel contract restart failed: $mode" }
    }
}
Write-Output "PASS: v20 neutral migration and v21 player/AI fuel continuation in four separate processes; test files: $directory"
