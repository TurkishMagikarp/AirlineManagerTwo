param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-slots-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Separate explicit save paths; each writer exits before its reader starts.
foreach ($version in @('v26', 'v27')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/slots_test.gd -- "--slots-mode=$mode" "--slots-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Slots restart failed: $mode" }
    }
}
Write-Output "PASS: v26 over-capacity migration and v27 player/AI slot continuation in four separate processes; files: $directory"
