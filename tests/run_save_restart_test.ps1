param(
    [string]$Godot = $env:AIRLINE_GODOT
)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$previousGodot = $env:AIRLINE_GODOT
try {
    $env:AIRLINE_GODOT = $Godot
    & (Join-Path $project 'START.bat') --headless --quit-after 1
    if ($LASTEXITCODE -ne 0) { throw 'START.bat failed' }
} finally {
    $env:AIRLINE_GODOT = $previousGodot
}
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-save-test-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
$save = Join-Path $directory 'Spielstand 日本語.json'
# Each invocation exits completely before the next starts. Never touches user://savegame.json.
foreach ($mode in @('write-running', 'read-running', 'write-paused', 'read-paused')) {
    & $Godot --headless --path $project --script res://tests/save_restart_test.gd -- "--save-test-mode=$mode" "--save-test-path=$save"
    if ($LASTEXITCODE -ne 0) { throw "Restart test failed: $mode" }
}
Write-Output "PASS: four separate Godot processes; test files: $directory"

# Named manual saves and autosave through the current dialogs: one writing process, then one fresh
# process per entry that loads it from the founding screen. Temporary profile incl. preferences.cfg.
$profile = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-slots-test-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $profile | Out-Null
foreach ($mode in @('write', 'read-a', 'read-b', 'read-auto')) {
    & $Godot --headless --path $project --script res://tests/save_slots_restart_test.gd -- "--save-test-mode=$mode" "--save-test-dir=$profile"
    if ($LASTEXITCODE -ne 0) { throw "Slot restart test failed: $mode" }
    # Real time passes while the application is closed; it must not move the game clock.
    Start-Sleep -Seconds 2
}
Write-Output "PASS: named saves and autosave across four more Godot processes; profile: $profile"
