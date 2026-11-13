param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-cabin-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Each process exits before the next starts; never touches user://savegame.json.
foreach ($version in @('v16', 'v17')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/cabin_test.gd -- "--cabin-mode=$mode" "--cabin-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Cabin restart failed: $mode" }
    }
}
Write-Output "PASS: v16 migration and v17 mixed cabin/fares in four separate processes; test files: $directory"
