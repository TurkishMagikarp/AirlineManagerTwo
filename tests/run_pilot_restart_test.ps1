param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-pilot-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Isolated explicit paths; each writer exits before its fresh reader starts.
foreach ($version in @('v24', 'v25')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/pilot_ratings_test.gd -- "--pilot-mode=$mode" "--pilot-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Pilot ratings restart failed: $mode" }
    }
}
Write-Output "PASS: v24 rating migration and v25 mid-training player/AI continuation in four separate processes; test files: $directory"
