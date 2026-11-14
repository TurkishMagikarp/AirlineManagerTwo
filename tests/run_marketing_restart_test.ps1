param([string]$Godot = $env:AIRLINE_GODOT)
$ErrorActionPreference = 'Stop'
if (-not $Godot) {
    $Godot = Join-Path $env:USERPROFILE 'Documents\Godot\Godot_v4.7.2-stable_win64_console.exe'
}
$project = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $env:LOCALAPPDATA ('Temp\opencode\airline-marketing-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $directory | Out-Null
# Each process exits before the next starts; never touches user://savegame.json.
foreach ($version in @('v19', 'v22')) {
    $save = Join-Path $directory ($version + '.json')
    foreach ($action in @('write', 'read')) {
        $mode = $action + '-' + $version
        & $Godot --headless --path $project --script res://tests/marketing_test.gd -- "--marketing-mode=$mode" "--marketing-save=$save"
        if ($LASTEXITCODE -ne 0) { throw "Marketing restart failed: $mode" }
    }
}
Write-Output "PASS: v19 migration and v22 mid-month marketing continuation in four separate processes; test files: $directory"
