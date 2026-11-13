param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-service-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Each process exits before the next starts; never touches user://savegame.json.
foreach ($version in @('v17', 'v18')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/service_test.gd -- "--service-mode=$mode" "--service-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Service restart failed: $mode" }
    }
}
Write-Output "PASS: v17 migration and v18 per-route service/frozen airborne costs in four separate processes; test files: $directory"
